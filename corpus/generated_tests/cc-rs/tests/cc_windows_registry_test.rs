use cc::windows_registry::{find, find_tool, find_vs_version};

#[test]
fn non_msvc_full_targets_do_not_use_windows_registry_lookup() {
    let targets = [
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
        "wasm32-unknown-unknown",
    ];

    for target in targets {
        assert!(
            find(target, "cl.exe").is_none(),
            "non-MSVC target {target} should not resolve cl.exe through the Windows registry"
        );
        assert!(
            find_tool(target, "cl.exe").is_none(),
            "non-MSVC target {target} should not resolve a Tool through the Windows registry"
        );
    }
}

#[test]
fn missing_tool_is_consistently_absent_for_supported_architecture_aliases() {
    let missing_tool = "__cc_rs_integration_test_missing_tool_7A0E2D19.exe";
    let arch_aliases = ["x64", "x86_64", "arm64", "aarch64", "x86", "i686"];

    for arch in arch_aliases {
        let command = find(arch, missing_tool);
        let tool = find_tool(arch, missing_tool);

        assert!(
            command.is_none(),
            "unexpectedly found missing command {missing_tool} for architecture alias {arch}"
        );
        assert!(
            tool.is_none(),
            "unexpectedly found missing tool {missing_tool} for architecture alias {arch}"
        );
        assert_eq!(
            command.is_some(),
            tool.is_some(),
            "find and find_tool should agree for missing tool {missing_tool} on {arch}"
        );
    }
}

#[test]
fn find_and_find_tool_agree_for_msvc_compiler_when_available() {
    let command = find("x86_64-pc-windows-msvc", "cl.exe");
    let tool = find_tool("x86_64-pc-windows-msvc", "cl.exe");

    assert_eq!(
        command.is_some(),
        tool.is_some(),
        "find and find_tool should agree on whether cl.exe is available"
    );

    if let Some(tool) = tool {
        assert!(
            !tool.path().as_os_str().is_empty(),
            "resolved tool path should not be empty"
        );

        let command_from_tool = tool.to_command();
        assert_eq!(
            command_from_tool.get_program(),
            tool.path().as_os_str(),
            "Tool::to_command should execute the resolved tool path"
        );

        assert!(
            !tool.cc_env().is_empty(),
            "resolved MSVC tool should expose a compiler environment value"
        );
        assert!(
            !tool.cflags_env().is_empty(),
            "resolved MSVC tool should expose a flags environment value"
        );
        assert!(
            tool.is_like_msvc()
                || tool.is_like_clang_cl()
                || tool.is_like_clang()
                || tool.is_like_gnu(),
            "resolved tool should identify at least one compiler family"
        );

        if let Some(command) = command {
            assert!(
                !command.get_program().is_empty(),
                "resolved command program should not be empty"
            );
        }
    } else {
        assert!(
            command.is_none(),
            "find should not return a command when find_tool cannot resolve the tool"
        );
    }
}

#[test]
fn visual_studio_version_lookup_reports_success_or_useful_failure() {
    let result = find_vs_version();

    match result {
        Ok(_) => {
            assert!(
                cfg!(windows),
                "Visual Studio version lookup should only succeed on Windows"
            );
        }
        Err(message) => {
            assert!(
                !message.trim().is_empty(),
                "Visual Studio version lookup failures should include a useful message"
            );
        }
    }
}