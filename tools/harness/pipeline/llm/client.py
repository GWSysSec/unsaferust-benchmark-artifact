"""
client.py — anthropic sdk client with opus/sonnet model routing

opus 4.7 handles analysis and test generation (brain).
sonnet handles compilation error fixing (fixer).
supports streaming, prompt caching, retry, and agentic tool-use loops.
"""

import re
import time
import json
import logging

from anthropic import Anthropic, APIStatusError
from pipeline.config import PipelineConfig, ModelConfig

log = logging.getLogger(__name__)

_HTML_RE = re.compile(r"^\s*(<\!DOCTYPE|<html|<\!--|<head)", re.IGNORECASE)

# upstream API provider periodically returns 503/504 during load. sleep a flat
# 10 min and retry, but cap at _GATEWAY_MAX_RETRIES per call so a sustained
# storm doesn't burn the entire per-crate timeout (this is what stranded
# async-lock and async-task in the first 96-crate run).
_GATEWAY_STATUS = {502, 503, 504}
_GATEWAY_WAIT_S = 600
_GATEWAY_MAX_RETRIES = 3

# 429: daily quota / rate limit hit. unlike a transient gateway hiccup, no
# point retrying inside the same session — the budget won't refill for hours.
# raise `QuotaExhausted` so the caller can mark the crate as deferred and
# move on to one whose pipeline doesn't need the LLM.
_QUOTA_STATUS = {429}
_QUOTA_HINTS = ("用户额度不足", "daily quota", "quota exceeded", "rate limit",
                "insufficient quota", "balance is insufficient")


class QuotaExhausted(Exception):
    """upstream returned 429 / daily-quota signal; abandon LLM work for this crate."""


def _is_html(text: str) -> bool:
    return bool(_HTML_RE.match(text.strip()[:200]))


def _classify_quota(exc: Exception) -> int | None:
    """return 429 if `exc` looks like a daily-quota/rate-limit signal; else None.

    matches both the SDK's structured status_code path and the user-facing
    Chinese/English hints that mytokenland.com emits in the response body.
    """
    sc = getattr(exc, "status_code", None)
    if isinstance(sc, int) and sc in _QUOTA_STATUS:
        return sc
    msg = str(exc)
    for code in _QUOTA_STATUS:
        if f" {code} " in msg or f"({code})" in msg or f"status {code}" in msg.lower():
            return code
    msg_lower = msg.lower()
    if any(h.lower() in msg_lower for h in _QUOTA_HINTS):
        return 429
    return None


def _classify_gateway(exc: Exception) -> int | None:
    """return the HTTP status if `exc` is a 502/503/504; else None.

    we check both the SDK's structured `status_code` attribute and the string
    form of the error, because the upstream relay sometimes wraps the response
    as an APIConnectionError (no .status_code) but the HTML body still mentions
    the code.
    """
    sc = getattr(exc, "status_code", None)
    if isinstance(sc, int) and sc in _GATEWAY_STATUS:
        return sc
    msg = str(exc)
    for code in _GATEWAY_STATUS:
        if f" {code} " in msg or f"({code})" in msg or f"status {code}" in msg.lower():
            return code
    return None


class LLMClient:
    def __init__(self, cfg: PipelineConfig):
        self.cfg = cfg
        base_url = cfg.base_url
        if base_url.endswith("/v1"):
            base_url = base_url[:-3]
        elif base_url.endswith("/v1/"):
            base_url = base_url[:-4]

        self.client = Anthropic(api_key=cfg.api_key, base_url=base_url)
        self.model_brain = cfg.model_brain
        self.model_fixer = cfg.model_fixer
        self.total_calls = 0
        self.total_latency = 0.0
        self._cache_headers = {"anthropic-beta": "prompt-caching-2024-07-31"}

    def _get_model(self, role: str) -> ModelConfig:
        if role == "brain":
            return self.model_brain
        return self.model_fixer

    def preflight(self) -> bool:
        log.info("  [llm] pre-flight check (anthropic)...")
        try:
            resp = self.client.messages.create(
                model=self.model_fixer.model,
                max_tokens=10,
                messages=[{"role": "user", "content": "ping"}],
            )
            log.info(f"  [llm] api reachable — model: {resp.model}")
            return True
        except Exception as e:
            log.warning(f"  [llm] pre-flight failed: {e}")
            return False

    def call(self, role: str, system: str, user: str) -> str:
        """single llm call. role='brain' routes to opus, 'fixer' to sonnet."""
        mcfg = self._get_model(role)
        messages = [{"role": "user", "content": user}]
        return self._invoke(role, mcfg, system, messages)

    def call_messages(self, role: str, system: str, messages: list[dict]) -> str:
        """multi-turn call with pre-built message list."""
        mcfg = self._get_model(role)
        return self._invoke(role, mcfg, system, messages)

    def _invoke(self, role: str, mcfg: ModelConfig, system: str,
                messages: list[dict]) -> str:
        last_error = ""
        attempt = 0
        gateway_retries = 0
        while attempt < 8:
            try:
                t0 = time.time()
                log.info(f"  [llm] {role}/{mcfg.model} streaming...", )

                with self.client.messages.stream(
                    model=mcfg.model,
                    system=system,
                    messages=messages,
                    temperature=mcfg.temperature,
                    max_tokens=mcfg.max_tokens,
                    extra_headers=self._cache_headers,
                ) as stream:
                    parts = []
                    for text in stream.text_stream:
                        parts.append(text)
                    content = "".join(parts)

                elapsed = time.time() - t0

                if _is_html(content):
                    wait = 2 ** (attempt + 1)
                    log.warning(f"  [llm] got HTML response — retrying in {wait}s")
                    last_error = "HTML error page"
                    time.sleep(wait)
                    attempt += 1
                    continue

                self.total_calls += 1
                self.total_latency += elapsed
                log.info(f"  [llm] done in {elapsed:.1f}s")
                return content

            except QuotaExhausted:
                raise
            except Exception as e:
                quota = _classify_quota(e)
                if quota is not None:
                    log.warning(f"  [llm] {quota} from upstream — daily quota signal; aborting call")
                    raise QuotaExhausted(f"http {quota}: {str(e)[:200]}") from e
                gw = _classify_gateway(e)
                if gw is not None:
                    if gateway_retries >= _GATEWAY_MAX_RETRIES:
                        last_error = f"gateway {gw} (cap {_GATEWAY_MAX_RETRIES} hit)"
                        log.warning(
                            f"  [llm] {gw} from upstream — gateway retry cap "
                            f"({_GATEWAY_MAX_RETRIES}) hit, giving up this call"
                        )
                        break
                    gateway_retries += 1
                    log.warning(
                        f"  [llm] {gw} from upstream — sleeping {_GATEWAY_WAIT_S}s "
                        f"then retrying (gateway retry {gateway_retries}/{_GATEWAY_MAX_RETRIES})"
                    )
                    time.sleep(_GATEWAY_WAIT_S)
                    continue
                wait = 2 ** (attempt + 1)
                last_error = str(e)[:300]
                log.warning(f"  [llm] attempt {attempt+1} failed: {last_error} — retrying in {wait}s")
                time.sleep(wait)
                attempt += 1

        log.error(f"  [llm] all attempts failed: {last_error}")
        return ""

    def call_agentic(self, system: str, messages: list[dict],
                     tools: list[dict], available_functions: dict,
                     max_turns: int = 15) -> str:
        """agentic multi-turn tool-use loop (brain model only)."""
        mcfg = self.model_brain
        current_messages = list(messages)

        anthropic_tools = []
        for t in tools:
            f = t["function"]
            anthropic_tools.append({
                "name": f["name"],
                "description": f.get("description", ""),
                "input_schema": f.get("parameters", {"type": "object", "properties": {}}),
            })

        text_only_retries = 0

        for turn in range(max_turns):
            log.info(f"  [agent] turn {turn+1} ({mcfg.model})...")
            t0 = time.time()
            response = None
            gateway_retries = 0
            while response is None:
                try:
                    response = self.client.messages.create(
                        model=mcfg.model,
                        system=system,
                        messages=current_messages,
                        temperature=mcfg.temperature,
                        max_tokens=mcfg.max_tokens,
                        tools=anthropic_tools,
                        extra_headers=self._cache_headers,
                    )
                except QuotaExhausted:
                    raise
                except Exception as e:
                    quota = _classify_quota(e)
                    if quota is not None:
                        log.warning(f"  [agent] {quota} from upstream — daily quota signal; aborting agent")
                        raise QuotaExhausted(f"http {quota}: {str(e)[:200]}") from e
                    gw = _classify_gateway(e)
                    if gw is not None:
                        if gateway_retries >= _GATEWAY_MAX_RETRIES:
                            log.error(
                                f"  [agent] {gw} from upstream — gateway retry "
                                f"cap ({_GATEWAY_MAX_RETRIES}) hit; aborting agent"
                            )
                            return f"error: llm api failed: gateway {gw} (cap hit)"
                        gateway_retries += 1
                        log.warning(
                            f"  [agent] {gw} from upstream — sleeping {_GATEWAY_WAIT_S}s "
                            f"then retrying turn ({gateway_retries}/{_GATEWAY_MAX_RETRIES})"
                        )
                        time.sleep(_GATEWAY_WAIT_S)
                        continue
                    log.error(f"  [agent] failed: {e}")
                    return f"error: llm api failed: {e}"
            log.info(f"  [agent] done in {time.time() - t0:.1f}s")
            self.total_calls += 1

            tool_uses = [b for b in response.content if b.type == "tool_use"]
            text_blocks = [b for b in response.content if b.type == "text"]

            if not tool_uses:
                text = text_blocks[0].text if text_blocks else ""

                # if the model returned text-only on an early turn, it may be
                # "thinking" without calling tools. nudge it to use tools.
                if turn < 3 and text_only_retries < 2 and not text.startswith(("SUCCESS:", "FAILURE:")):
                    text_only_retries += 1
                    log.warning(f"  [agent] got text-only response on turn {turn+1}, nudging to use tools...")
                    # only append assistant message if it has content
                    # (some proxies strip content blocks, causing 400 errors)
                    if response.content:
                        current_messages.append({"role": "assistant", "content": response.content})
                    else:
                        current_messages.append({"role": "assistant", "content": [{"type": "text", "text": text or "I will use the tools now."}]})
                    current_messages.append({
                        "role": "user",
                        "content": "You must call one of the available tools now. "
                                   "Start with get_workspace_info or run_cargo_llvm_cov. "
                                   "Do NOT respond with only text.",
                    })
                    continue

                return text

            current_messages.append({"role": "assistant", "content": response.content})

            for tool_use in tool_uses:
                name = tool_use.name
                args = tool_use.input
                log.info(f"    -> calling {name}(...)")
                if name in available_functions:
                    try:
                        result = available_functions[name](**args)
                        result_str = json.dumps(result) if not isinstance(result, str) else result
                    except Exception as e:
                        result_str = f"error executing {name}: {e}"
                else:
                    result_str = f"error: tool {name} not found"

                current_messages.append({
                    "role": "user",
                    "content": [{"type": "tool_result", "tool_use_id": tool_use.id, "content": result_str}],
                })

        return "error: max agent turns reached"

    def stats(self) -> str:
        avg = self.total_latency / max(self.total_calls, 1)
        return f"llm stats: {self.total_calls} calls, {self.total_latency:.1f}s total, {avg:.1f}s avg"
