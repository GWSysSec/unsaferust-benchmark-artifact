use cc::windows_registry;




#[test]
fn test_windows_registry_find_returns_none_on_linux() {

    let result_x86 = windows_registry::find("x86", "cl.exe");
    assert!(result_x86.is_none(), "find should return None on Linux for x86 cl.exe");

    let result_x64 = windows_registry::find("x86_64", "cl.exe");
    assert!(result_x64.is_none(), "find should return None on Linux for x86_64 cl.exe");

    let result_aarch64 = windows_registry::find("aarch64", "cl.exe");
    assert!(result_aarch64.is_none(), "find should return None on Linux for aarch64 cl.exe");


    let result_link_x86 = windows_registry::find("x86", "link.exe");
    assert!(result_link_x86.is_none(), "find should return None on Linux for x86 link.exe");

    let result_link_x64 = windows_registry::find("x86_64", "link.exe");
    assert!(result_link_x64.is_none(), "find should return None on Linux for x86_64 link.exe");


    let result_lib = windows_registry::find("x86", "lib.exe");
    assert!(result_lib.is_none(), "find should return None on Linux for x86 lib.exe");


    let result_triple = windows_registry::find("x86_64-pc-windows-msvc", "cl.exe");
    assert!(result_triple.is_none(), "find should return None on Linux for full target triple");


    let result_empty = windows_registry::find("x86_64", "");
    assert!(result_empty.is_none(), "find should return None for empty tool name");
}

#[test]
fn test_windows_registry_find_tool_returns_none_on_linux() {

    let tool_x86 = windows_registry::find_tool("x86", "cl.exe");
    assert!(tool_x86.is_none(), "find_tool should return None on Linux for x86 cl.exe");

    let tool_x64 = windows_registry::find_tool("x86_64", "cl.exe");
    assert!(tool_x64.is_none(), "find_tool should return None on Linux for x86_64 cl.exe");

    let tool_aarch64 = windows_registry::find_tool("aarch64", "cl.exe");
    assert!(tool_aarch64.is_none(), "find_tool should return None on Linux for aarch64 cl.exe");


    let tool_link = windows_registry::find_tool("x86", "link.exe");
    assert!(tool_link.is_none(), "find_tool should return None on Linux for x86 link.exe");

    let tool_link_x64 = windows_registry::find_tool("x86_64", "link.exe");
    assert!(tool_link_x64.is_none(), "find_tool should return None on Linux for x86_64 link.exe");


    let tool_lib = windows_registry::find_tool("x86_64", "lib.exe");
    assert!(tool_lib.is_none(), "find_tool should return None on Linux for x86_64 lib.exe");


    let tool_triple = windows_registry::find_tool("x86_64-pc-windows-msvc", "cl.exe");
    assert!(tool_triple.is_none(), "find_tool should return None on Linux for full target triple");


    let tool_i686 = windows_registry::find_tool("i686", "cl.exe");
    assert!(tool_i686.is_none(), "find_tool should return None on Linux for i686 cl.exe");
}

#[test]
fn test_windows_registry_find_vs_version_on_linux() {

    let result = windows_registry::find_vs_version();


    assert!(result.is_err(), "find_vs_version should return Err on Linux");


    let err_msg = result.unwrap_err();
    assert!(!err_msg.is_empty(), "Error message should not be empty");


    let result2 = windows_registry::find_vs_version();
    assert!(result2.is_err(), "find_vs_version should consistently return Err on Linux");

    let err_msg2 = result2.unwrap_err();
    assert!(!err_msg2.is_empty(), "Second error message should not be empty");


    assert_eq!(err_msg, err_msg2, "Error messages should be consistent across calls");


    assert!(err_msg.len() > 3, "Error message should be descriptive, got: {}", err_msg);


    let cloned_err = err_msg.clone();
    assert_eq!(cloned_err, err_msg, "Cloned error should equal original");
}

#[test]
fn test_windows_registry_find_various_architectures() {

    let architectures = [
        "x86",
        "x86_64",
        "aarch64",
        "i686",
        "arm",
        "x86_64-pc-windows-msvc",
        "i686-pc-windows-msvc",
        "aarch64-pc-windows-msvc",
    ];

    let tools = ["cl.exe", "link.exe", "lib.exe", "ml.exe", "ml64.exe"];

    let mut none_count = 0u32;

    for arch in &architectures {
        for tool in &tools {
            let result = windows_registry::find(arch, tool);
            if result.is_none() {
                none_count += 1;
            }
        }
    }


    let total = (architectures.len() * tools.len()) as u32;
    assert_eq!(none_count, total, "All find calls should return None on Linux");
    assert!(none_count >= 8, "Should have tested at least 8 combinations");


    let mut tool_none_count = 0u32;
    for arch in &architectures {
        for tool in &tools {
            let result = windows_registry::find_tool(arch, tool);
            if result.is_none() {
                tool_none_count += 1;
            }
        }
    }

    assert_eq!(tool_none_count, total, "All find_tool calls should return None on Linux");
    assert_eq!(none_count, tool_none_count, "find and find_tool should agree");
}

#[test]
fn test_windows_registry_find_tool_consistency() {

    let arch = "x86_64";
    let tool_name = "cl.exe";

    let result1 = windows_registry::find_tool(arch, tool_name);
    let result2 = windows_registry::find_tool(arch, tool_name);
    let result3 = windows_registry::find_tool(arch, tool_name);

    assert_eq!(result1.is_none(), result2.is_none(), "Consecutive calls should be consistent");
    assert_eq!(result2.is_none(), result3.is_none(), "All three calls should agree");


    assert!(result1.is_none(), "Should be None on Linux");
    assert!(result2.is_none(), "Should be None on Linux (2nd call)");
    assert!(result3.is_none(), "Should be None on Linux (3rd call)");


    let find_result1 = windows_registry::find(arch, tool_name);
    let find_result2 = windows_registry::find(arch, tool_name);

    assert!(find_result1.is_none(), "find should be None on Linux");
    assert!(find_result2.is_none(), "find should be None on Linux (2nd call)");
}

#[test]
fn test_windows_registry_find_with_unusual_inputs() {

    let result_empty_arch = windows_registry::find("", "cl.exe");
    assert!(result_empty_arch.is_none(), "Empty arch should return None");

    let result_empty_both = windows_registry::find("", "");
    assert!(result_empty_both.is_none(), "Empty arch and tool should return None");

    let result_spaces = windows_registry::find("  ", "cl.exe");
    assert!(result_spaces.is_none(), "Whitespace arch should return None");

    let result_long_arch = windows_registry::find("x86_64-pc-windows-msvc-some-extra-stuff", "cl.exe");
    assert!(result_long_arch.is_none(), "Long arch string should return None on Linux");


    let tool_empty_arch = windows_registry::find_tool("", "cl.exe");
    assert!(tool_empty_arch.is_none(), "find_tool with empty arch should return None");

    let tool_empty_both = windows_registry::find_tool("", "");
    assert!(tool_empty_both.is_none(), "find_tool with empty inputs should return None");

    let tool_spaces = windows_registry::find_tool("  ", "cl.exe");
    assert!(tool_spaces.is_none(), "find_tool with whitespace arch should return None");

    let tool_long = windows_registry::find_tool("x86_64-pc-windows-msvc-some-extra-stuff", "cl.exe");
    assert!(tool_long.is_none(), "find_tool with long arch should return None on Linux");
}