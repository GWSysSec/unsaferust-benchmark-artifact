use serde::{Deserialize, Serialize};
use serde_with::rust::deserialize_ignore_any;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Message {
    Ping { id: u32 },
    Pong { id: u32 },
    #[serde(other, deserialize_with = "deserialize_ignore_any")]
    Unknown,
}

#[test]
fn ignore_any_unknown_variants() {
    let ping: Message = serde_json::from_str(r#"{"type":"ping","id":1}"#).unwrap();
    assert_eq!(ping, Message::Ping { id: 1 });

    let pong: Message = serde_json::from_str(r#"{"type":"pong","id":2}"#).unwrap();
    assert_eq!(pong, Message::Pong { id: 2 });


    let unknown1: Message = serde_json::from_str(r#"{"type":"foo","id":3}"#).unwrap();
    assert_eq!(unknown1, Message::Unknown);


    let unknown2: Message =
        serde_json::from_str(r#"{"type":"bar","data":{"a":[1,2,3],"b":null}}"#).unwrap();
    assert_eq!(unknown2, Message::Unknown);


    let unknown3: Message =
        serde_json::from_str(r#"{"type":"baz","arr":[true,false,null,"x",1.5]}"#).unwrap();
    assert_eq!(unknown3, Message::Unknown);


    let deep = r#"{"type":"deep","x":{"y":{"z":[{"a":1},{"b":[null,null]}]}}}"#;
    let unknown4: Message = serde_json::from_str(deep).unwrap();
    assert_eq!(unknown4, Message::Unknown);


    let list_json = r#"[
        {"type":"ping","id":10},
        {"type":"weird","payload":42},
        {"type":"pong","id":20},
        {"type":"unknown_thing","stuff":"abc"}
    ]"#;
    let list: Vec<Message> = serde_json::from_str(list_json).unwrap();
    assert_eq!(list.len(), 4);
    assert_eq!(list[0], Message::Ping { id: 10 });
    assert_eq!(list[1], Message::Unknown);
    assert_eq!(list[2], Message::Pong { id: 20 });
    assert_eq!(list[3], Message::Unknown);
    assert_ne!(list[0], list[1]);
}

#[test]
fn ignore_any_in_struct_field() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Wrapper {
        name: String,
        #[serde(deserialize_with = "deserialize_ignore_any", default)]
        ignored: (),
        count: i32,
    }

    let w1: Wrapper =
        serde_json::from_str(r#"{"name":"a","ignored":{"complex":[1,2,3]},"count":5}"#).unwrap();
    assert_eq!(w1.name, "a");
    assert_eq!(w1.ignored, ());
    assert_eq!(w1.count, 5);

    let w2: Wrapper = serde_json::from_str(r#"{"name":"b","ignored":null,"count":-7}"#).unwrap();
    assert_eq!(w2.name, "b");
    assert_eq!(w2.count, -7);
    assert_ne!(w2.name, w1.name);

    let w3: Wrapper = serde_json::from_str(r#"{"name":"c","ignored":"any string","count":0}"#).unwrap();
    assert_eq!(w3.name, "c");
    assert_eq!(w3.count, 0);

    let w4: Wrapper =
        serde_json::from_str(r#"{"name":"d","ignored":[1,[2,[3,[4]]]],"count":99}"#).unwrap();
    assert_eq!(w4.count, 99);
    assert_eq!(w4.ignored, ());
}

#[test]
fn ignore_any_invalid_json_still_errors() {

    let result: Result<Message, _> = serde_json::from_str(r#"{"type":"foo","x":"#);
    assert!(result.is_err());

    let valid: Result<Message, _> = serde_json::from_str(r#"{"type":"foo","x":1}"#);
    assert!(valid.is_ok());
    assert_eq!(valid.unwrap(), Message::Unknown);


    #[derive(Serialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    enum Out {
        Ping { id: u32 },
    }
    let s = serde_json::to_string(&Out::Ping { id: 7 }).unwrap();
    assert!(s.contains("\"ping\""));
    assert!(s.contains("\"id\":7"));
    let back: Message = serde_json::from_str(&s).unwrap();
    assert_eq!(back, Message::Ping { id: 7 });
}