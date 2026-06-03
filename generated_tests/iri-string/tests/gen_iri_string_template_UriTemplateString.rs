use iri_string::template::{UriTemplateStr, UriTemplateString};

#[test]
fn test_new_unchecked_valid_template() {

    let valid_template = String::from("http://example.com/{var}");
    let template = unsafe { UriTemplateString::new_unchecked(valid_template) };
    let slice = template.as_slice();
    assert_eq!(slice.as_str(), "http://example.com/{var}");
    assert_eq!(template.as_slice().as_str(), "http://example.com/{var}");


    let cap = template.capacity();
    assert!(cap >= 24, "capacity should be at least string length, got {}", cap);


    let simple = String::from("{host}");
    let t2 = unsafe { UriTemplateString::new_unchecked(simple) };
    assert_eq!(t2.as_slice().as_str(), "{host}");
    assert!(t2.capacity() >= 6);


    let empty = String::from("");
    let t3 = unsafe { UriTemplateString::new_unchecked(empty) };
    assert_eq!(t3.as_slice().as_str(), "");
    assert_eq!(t3.as_slice().as_str().len(), 0);
}

#[test]
fn test_shrink_to_fit_reduces_capacity() {

    let mut s = String::with_capacity(1024);
    s.push_str("http://example.com/{id}");
    let original_len = s.len();

    let mut template = unsafe { UriTemplateString::new_unchecked(s) };


    let cap_before = template.capacity();
    assert!(cap_before >= 1024, "expected large capacity before shrink, got {}", cap_before);
    assert_eq!(template.as_slice().as_str().len(), original_len);


    template.shrink_to_fit();


    let cap_after = template.capacity();
    assert!(cap_after <= cap_before, "capacity should not grow after shrink_to_fit");
    assert!(cap_after >= original_len, "capacity must be at least string length");

    assert_eq!(template.as_slice().as_str(), "http://example.com/{id}");


    template.shrink_to_fit();
    let cap_after2 = template.capacity();
    assert_eq!(cap_after2, cap_after);
}

#[test]
fn test_capacity_reflects_underlying_allocation() {
    let s = String::from("http://{host}/{path}");
    let len = s.len();
    let template = unsafe { UriTemplateString::new_unchecked(s) };

    let cap = template.capacity();
    assert!(cap >= len, "capacity {} must be >= len {}", cap, len);
    assert_eq!(template.as_slice().as_str().len(), len);


    let mut big = String::with_capacity(2048);
    big.push_str("/a");
    let t2 = unsafe { UriTemplateString::new_unchecked(big) };
    assert!(t2.capacity() >= 2048);
    assert_eq!(t2.as_slice().as_str(), "/a");


    let empty = String::new();
    let t3 = unsafe { UriTemplateString::new_unchecked(empty) };
    assert!(t3.capacity() >= 0);
    assert_eq!(t3.as_slice().as_str(), "");
}

#[test]
fn test_as_slice_returns_correct_reference() {
    let input = "http://example.org/{+path}/here";
    let template = unsafe { UriTemplateString::new_unchecked(String::from(input)) };

    let slice: &UriTemplateStr = template.as_slice();
    assert_eq!(slice.as_str(), input);
    assert_eq!(slice.as_str().len(), input.len());


    let slice2 = template.as_slice();
    assert_eq!(slice.as_str(), slice2.as_str());
    assert_eq!(std::ptr::eq(slice, slice2), true);


    assert!(slice.as_str().starts_with("http://"));
    assert!(slice.as_str().contains("{+path}"));
    assert!(slice.as_str().ends_with("/here"));
}

#[test]
fn test_append_concatenates_templates() {
    let base_str = "http://example.com";
    let mut template = unsafe { UriTemplateString::new_unchecked(String::from(base_str)) };

    assert_eq!(template.as_slice().as_str(), "http://example.com");


    let suffix = UriTemplateStr::new("/{resource}").expect("valid template");
    template.append(suffix);

    assert_eq!(template.as_slice().as_str(), "http://example.com/{resource}");
    assert!(template.capacity() >= 30);


    let suffix2 = UriTemplateStr::new("/{id}").expect("valid template");
    template.append(suffix2);

    assert_eq!(template.as_slice().as_str(), "http://example.com/{resource}/{id}");


    let empty_suffix = UriTemplateStr::new("").expect("valid template");
    let len_before = template.as_slice().as_str().len();
    template.append(empty_suffix);
    assert_eq!(template.as_slice().as_str().len(), len_before);
    assert_eq!(template.as_slice().as_str(), "http://example.com/{resource}/{id}");
}

#[test]
fn test_append_multiple_segments_workflow() {

    let mut template = unsafe { UriTemplateString::new_unchecked(String::from("https://")) };

    let host_part = UriTemplateStr::new("{host}").expect("valid");
    template.append(host_part);
    assert_eq!(template.as_slice().as_str(), "https://{host}");

    let port_part = UriTemplateStr::new(":{port}").expect("valid");
    template.append(port_part);
    assert_eq!(template.as_slice().as_str(), "https://{host}:{port}");

    let path_part = UriTemplateStr::new("/api/v1/{endpoint}").expect("valid");
    template.append(path_part);
    assert_eq!(template.as_slice().as_str(), "https://{host}:{port}/api/v1/{endpoint}");

    let query_part = UriTemplateStr::new("{?page,limit}").expect("valid");
    template.append(query_part);

    let final_str = template.as_slice().as_str().to_owned();

    assert_eq!(final_str, "https://{host}:{port}/api/v1/{endpoint}{?page,limit}");
    assert!(template.capacity() >= final_str.len());


    template.shrink_to_fit();
    assert_eq!(template.as_slice().as_str(), final_str);
    assert!(template.capacity() >= final_str.len());
}

#[test]
fn test_new_unchecked_with_various_expressions() {

    let t1 = unsafe { UriTemplateString::new_unchecked(String::from("{var}")) };
    assert_eq!(t1.as_slice().as_str(), "{var}");


    let t2 = unsafe { UriTemplateString::new_unchecked(String::from("{+path}")) };
    assert_eq!(t2.as_slice().as_str(), "{+path}");


    let t3 = unsafe { UriTemplateString::new_unchecked(String::from("{x,y}")) };
    assert_eq!(t3.as_slice().as_str(), "{x,y}");


    let t4 = unsafe { UriTemplateString::new_unchecked(String::from("{/var,x}")) };
    assert_eq!(t4.as_slice().as_str(), "{/var,x}");


    let t5 = unsafe { UriTemplateString::new_unchecked(String::from("{?x,y}")) };
    assert_eq!(t5.as_slice().as_str(), "{?x,y}");


    let t6 = unsafe { UriTemplateString::new_unchecked(String::from("{#path}")) };
    assert_eq!(t6.as_slice().as_str(), "{#path}");


    let t7 = unsafe { UriTemplateString::new_unchecked(String::from("{.who}")) };
    assert_eq!(t7.as_slice().as_str(), "{.who}");


    let t8 = unsafe { UriTemplateString::new_unchecked(String::from("{.who:5}")) };
    assert_eq!(t8.as_slice().as_str(), "{.who:5}");
}

#[test]
fn test_capacity_growth_through_append() {
    let mut template = unsafe { UriTemplateString::new_unchecked(String::from("x")) };
    let initial_cap = template.capacity();
    assert!(initial_cap >= 1);


    let long_segment = UriTemplateStr::new(
        "/this/is/a/fairly/long/path/segment/that/should/force/reallocation"
    ).expect("valid template literal");

    template.append(long_segment);

    let new_cap = template.capacity();
    assert!(new_cap >= template.as_slice().as_str().len());
    assert!(template.as_slice().as_str().starts_with("x/this/is/a/fairly"));
    assert!(template.as_slice().as_str().len() > 60);


    template.shrink_to_fit();
    let shrunk_cap = template.capacity();
    assert!(shrunk_cap >= template.as_slice().as_str().len());
    assert!(shrunk_cap <= new_cap);
}

#[test]
fn test_roundtrip_new_unchecked_matches_safe_new() {
    let input = "http://example.com/{user}/profile{?fields}";


    let safe_ref = UriTemplateStr::new(input).expect("valid template");


    let unsafe_owned = unsafe { UriTemplateString::new_unchecked(String::from(input)) };


    assert_eq!(safe_ref.as_str(), unsafe_owned.as_slice().as_str());
    assert_eq!(safe_ref.as_str().len(), unsafe_owned.as_slice().as_str().len());


    assert_eq!(
        safe_ref.as_str().as_bytes(),
        unsafe_owned.as_slice().as_str().as_bytes()
    );


    assert!(unsafe_owned.capacity() >= input.len());


    assert!(unsafe_owned.as_slice().as_str().contains("{user}"));
    assert!(unsafe_owned.as_slice().as_str().contains("{?fields}"));
    assert!(unsafe_owned.as_slice().as_str().starts_with("http://"));
    assert!(unsafe_owned.as_slice().as_str().ends_with("{?fields}"));
}