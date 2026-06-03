use iri_string::template::UriTemplateStr;
use iri_string::template::UriTemplateString;
use iri_string::template::UriTemplateVariables;
use iri_string::template::simple_context::SimpleContext;
use iri_string::spec::UriSpec;

#[test]
fn test_uri_template_str_new_unchecked_basic() {

    let template_str = "http://example.com/{var}";
    let result = UriTemplateStr::new(template_str);
    assert!(result.is_ok());
    let validated = result.unwrap();



    let unchecked = unsafe { UriTemplateStr::new_unchecked(template_str) };


    assert_eq!(validated.as_str(), unchecked.as_str());
    assert_eq!(unchecked.as_str(), "http://example.com/{var}");


    let complex = "http://{host}/{path}{?query}";
    let validated_complex = UriTemplateStr::new(complex).unwrap();
    let unchecked_complex = unsafe { UriTemplateStr::new_unchecked(complex) };
    assert_eq!(validated_complex.as_str(), unchecked_complex.as_str());
    assert_eq!(unchecked_complex.as_str(), complex);


    let simple = "http://example.com/";
    let unchecked_simple = unsafe { UriTemplateStr::new_unchecked(simple) };
    assert_eq!(unchecked_simple.as_str(), "http://example.com/");


    let multi = "{scheme}://{host}/{+path}";
    let unchecked_multi = unsafe { UriTemplateStr::new_unchecked(multi) };
    assert_eq!(unchecked_multi.as_str(), multi);
}

#[test]
fn test_uri_template_str_new_unchecked_various_operators() {

    let reserved = "{+path}/here";
    let unchecked = unsafe { UriTemplateStr::new_unchecked(reserved) };
    assert_eq!(unchecked.as_str(), "{+path}/here");


    let fragment = "http://example.com/base{#fragment}";
    let unchecked_frag = unsafe { UriTemplateStr::new_unchecked(fragment) };
    assert_eq!(unchecked_frag.as_str(), fragment);


    let label = "http://example.com/base{.ext}";
    let unchecked_label = unsafe { UriTemplateStr::new_unchecked(label) };
    assert_eq!(unchecked_label.as_str(), label);


    let path_seg = "http://example.com{/path}";
    let unchecked_path = unsafe { UriTemplateStr::new_unchecked(path_seg) };
    assert_eq!(unchecked_path.as_str(), path_seg);


    let query = "http://example.com/search{?q,lang}";
    let unchecked_query = unsafe { UriTemplateStr::new_unchecked(query) };
    assert_eq!(unchecked_query.as_str(), query);


    let query_cont = "http://example.com/search?q=test{&page,limit}";
    let unchecked_qc = unsafe { UriTemplateStr::new_unchecked(query_cont) };
    assert_eq!(unchecked_qc.as_str(), query_cont);


    assert!(UriTemplateStr::new(reserved).is_ok());
    assert!(UriTemplateStr::new(fragment).is_ok());
}

#[test]
fn test_uri_template_str_variables_single_var() {
    let template = UriTemplateStr::new("http://example.com/{username}/profile").unwrap();
    let vars: UriTemplateVariables<'_> = template.variables();

    let var_names: Vec<_> = vars.collect();
    assert_eq!(var_names.len(), 1);
    assert_eq!(var_names[0].as_str(), "username");
}

#[test]
fn test_uri_template_str_variables_multiple_vars() {
    let template = UriTemplateStr::new("http://{host}:{port}/{path}{?query}").unwrap();
    let vars = template.variables();

    let var_names: Vec<_> = vars.collect();
    assert_eq!(var_names.len(), 4);
    assert_eq!(var_names[0].as_str(), "host");
    assert_eq!(var_names[1].as_str(), "port");
    assert_eq!(var_names[2].as_str(), "path");
    assert_eq!(var_names[3].as_str(), "query");
}

#[test]
fn test_uri_template_str_variables_no_vars() {
    let template = UriTemplateStr::new("http://example.com/static/path").unwrap();
    let vars = template.variables();

    let var_names: Vec<_> = vars.collect();
    assert_eq!(var_names.len(), 0);
}

#[test]
fn test_uri_template_str_variables_comma_separated() {
    let template = UriTemplateStr::new("{?x,y,z}").unwrap();
    let vars = template.variables();

    let var_names: Vec<_> = vars.collect();
    assert_eq!(var_names.len(), 3);
    assert_eq!(var_names[0].as_str(), "x");
    assert_eq!(var_names[1].as_str(), "y");
    assert_eq!(var_names[2].as_str(), "z");
}

#[test]
fn test_uri_template_str_variables_clone() {
    let template = UriTemplateStr::new("http://{host}/{path}").unwrap();
    let vars = template.variables();
    let vars_clone = vars.clone();

    let names1: Vec<_> = vars.collect();
    let names2: Vec<_> = vars_clone.collect();

    assert_eq!(names1.len(), names2.len());
    assert_eq!(names1.len(), 2);
    assert_eq!(names1[0].as_str(), names2[0].as_str());
    assert_eq!(names1[1].as_str(), names2[1].as_str());
    assert_eq!(names1[0].as_str(), "host");
    assert_eq!(names1[1].as_str(), "path");
}

#[test]
fn test_uri_template_str_expand_dynamic_simple() {
    let template = UriTemplateStr::new("http://example.com/{name}").unwrap();

    let mut ctx = SimpleContext::new();
    ctx.insert("name", "hello");

    let mut output = String::new();
    let result = template.expand_dynamic::<UriSpec, _, _>(&mut output, &mut ctx);
    assert!(result.is_ok());
    assert_eq!(output, "http://example.com/hello");
}

#[test]
fn test_uri_template_str_expand_dynamic_multiple_vars() {
    let template = UriTemplateStr::new("{scheme}://{host}/{path}").unwrap();

    let mut ctx = SimpleContext::new();
    ctx.insert("scheme", "https");
    ctx.insert("host", "example.org");
    ctx.insert("path", "resource");

    let mut output = String::new();
    let result = template.expand_dynamic::<UriSpec, _, _>(&mut output, &mut ctx);
    assert!(result.is_ok());
    assert_eq!(output, "https://example.org/resource");
}

#[test]
fn test_uri_template_str_expand_dynamic_query_expansion() {
    let template = UriTemplateStr::new("http://example.com/search{?q,lang}").unwrap();

    let mut ctx = SimpleContext::new();
    ctx.insert("q", "rust");
    ctx.insert("lang", "en");

    let mut output = String::new();
    let result = template.expand_dynamic::<UriSpec, _, _>(&mut output, &mut ctx);
    assert!(result.is_ok());
    assert_eq!(output, "http://example.com/search?q=rust&lang=en");
}

#[test]
fn test_uri_template_str_expand_dynamic_missing_var() {
    let template = UriTemplateStr::new("http://example.com/{name}").unwrap();

    let mut ctx = SimpleContext::new();


    let mut output = String::new();
    let result = template.expand_dynamic::<UriSpec, _, _>(&mut output, &mut ctx);
    assert!(result.is_ok());
    assert_eq!(output, "http://example.com/");
}

#[test]
fn test_uri_template_str_expand_dynamic_no_vars_in_template() {
    let template = UriTemplateStr::new("http://example.com/static").unwrap();

    let mut ctx = SimpleContext::new();

    let mut output = String::new();
    let result = template.expand_dynamic::<UriSpec, _, _>(&mut output, &mut ctx);
    assert!(result.is_ok());
    assert_eq!(output, "http://example.com/static");
}

#[test]
fn test_uri_template_str_expand_dynamic_with_unchecked() {

    let template_str = "http://{host}/api/{version}/users";

    let template = unsafe { UriTemplateStr::new_unchecked(template_str) };

    let mut ctx = SimpleContext::new();
    ctx.insert("host", "api.example.com");
    ctx.insert("version", "v2");

    let mut output = String::new();
    let result = template.expand_dynamic::<UriSpec, _, _>(&mut output, &mut ctx);
    assert!(result.is_ok());
    assert_eq!(output, "http://api.example.com/api/v2/users");


    let vars: Vec<_> = template.variables().collect();
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].as_str(), "host");
    assert_eq!(vars[1].as_str(), "version");
}

#[test]
fn test_uri_template_str_expand_dynamic_percent_encoding() {
    let template = UriTemplateStr::new("http://example.com/{path}").unwrap();

    let mut ctx = SimpleContext::new();
    ctx.insert("path", "hello world");

    let mut output = String::new();
    let result = template.expand_dynamic::<UriSpec, _, _>(&mut output, &mut ctx);
    assert!(result.is_ok());

    assert_eq!(output, "http://example.com/hello%20world");
}

#[test]
fn test_uri_template_string_new_unchecked_and_methods() {

    let s = String::from("http://example.com/{resource}/{id}");

    let template_string = unsafe { UriTemplateString::new_unchecked(s) };


    let template_ref: &UriTemplateStr = template_string.as_ref();
    assert_eq!(template_ref.as_str(), "http://example.com/{resource}/{id}");


    let vars: Vec<_> = template_ref.variables().collect();
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].as_str(), "resource");
    assert_eq!(vars[1].as_str(), "id");


    let mut ctx = SimpleContext::new();
    ctx.insert("resource", "books");
    ctx.insert("id", "42");

    let mut output = String::new();
    let result = template_ref.expand_dynamic::<UriSpec, _, _>(&mut output, &mut ctx);
    assert!(result.is_ok());
    assert_eq!(output, "http://example.com/books/42");
}

#[test]
fn test_uri_template_str_variables_with_modifiers() {

    let template = UriTemplateStr::new("{var:3}/{name}").unwrap();
    let vars: Vec<_> = template.variables().collect();

    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].as_str(), "var");
    assert_eq!(vars[1].as_str(), "name");


    let template2 = UriTemplateStr::new("{/list*}").unwrap();
    let vars2: Vec<_> = template2.variables().collect();
    assert_eq!(vars2.len(), 1);
    assert_eq!(vars2[0].as_str(), "list");
}

#[test]
fn test_uri_template_workflow_end_to_end() {

    let raw = "https://{host}/api/{version}/users/{user_id}{?fields,format}";


    let template = UriTemplateStr::new(raw).unwrap();
    assert_eq!(template.as_str(), raw);


    let vars: Vec<_> = template.variables().collect();
    assert_eq!(vars.len(), 5);
    assert_eq!(vars[0].as_str(), "host");
    assert_eq!(vars[1].as_str(), "version");
    assert_eq!(vars[2].as_str(), "user_id");
    assert_eq!(vars[3].as_str(), "fields");
    assert_eq!(vars[4].as_str(), "format");


    let mut ctx = SimpleContext::new();
    ctx.insert("host", "api.example.com");
    ctx.insert("version", "v1");
    ctx.insert("user_id", "123");
    ctx.insert("fields", "name,email");
    ctx.insert("format", "json");

    let mut output = String::new();
    let result = template.expand_dynamic::<UriSpec, _, _>(&mut output, &mut ctx);
    assert!(result.is_ok());
    assert!(output.starts_with("https://api.example.com/api/v1/users/123"));
    assert!(output.contains("fields="));
    assert!(output.contains("format=json"));
}