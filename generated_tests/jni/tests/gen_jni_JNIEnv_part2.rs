#![cfg(feature = "invocation")]

use std::ffi::c_void;

use jni::{objects::JValue, NativeMethod};

mod util;
use util::jvm;

#[test]
fn test_get_set_field_int() {
    let mut env = jvm().attach_current_thread().unwrap();


    let obj = env
        .new_object(
            "java/util/concurrent/atomic/AtomicInteger",
            "(I)V",
            &[JValue::Int(42)],
        )
        .unwrap();
    assert_eq!(obj.is_null(), false);


    let v0 = env.get_field(&obj, "value", "I").unwrap();
    assert_eq!(v0.i().unwrap(), 42i32);


    let call0 = env.call_method(&obj, "get", "()I", &[]).unwrap();
    assert_eq!(call0.i().unwrap(), 42i32);


    env.set_field(&obj, "value", "I", JValue::Int(100)).unwrap();

    let v1a = env.get_field(&obj, "value", "I").unwrap();
    assert_eq!(v1a.i().unwrap(), 100i32);
    let v1b = env.get_field(&obj, "value", "I").unwrap();
    assert_ne!(v1b.i().unwrap(), 42i32);

    let call1 = env.call_method(&obj, "get", "()I", &[]).unwrap();
    assert_eq!(call1.i().unwrap(), 100i32);


    env.set_field(&obj, "value", "I", JValue::Int(i32::MIN))
        .unwrap();
    let v2 = env.get_field(&obj, "value", "I").unwrap();
    assert_eq!(v2.i().unwrap(), i32::MIN);


    env.set_field(&obj, "value", "I", JValue::Int(0)).unwrap();
    let v3 = env.get_field(&obj, "value", "I").unwrap();
    assert_eq!(v3.i().unwrap(), 0i32);


    let err = env.get_field(&obj, "not_a_field_abc", "I");
    assert!(err.is_err());
    if env.exception_check().unwrap_or(false) {
        env.exception_clear().unwrap();
    }


    let err2 = env.get_field(&obj, "value", "J");
    assert!(err2.is_err());
    if env.exception_check().unwrap_or(false) {
        env.exception_clear().unwrap();
    }
}

#[test]
fn test_set_static_field_object() {
    let mut env = jvm().attach_current_thread().unwrap();

    let out = env
        .get_static_field("java/lang/System", "out", "Ljava/io/PrintStream;")
        .unwrap()
        .l()
        .unwrap();
    assert_eq!(out.is_null(), false);

    let err_original = env
        .get_static_field("java/lang/System", "err", "Ljava/io/PrintStream;")
        .unwrap()
        .l()
        .unwrap();
    assert_eq!(err_original.is_null(), false);


    let same_initially = env.is_same_object(&out, &err_original).unwrap();
    assert_eq!(same_initially, false);


    env.set_static_field(
        "java/lang/System",
        ("java/lang/System", "err", "Ljava/io/PrintStream;"),
        JValue::Object(&out),
    )
    .unwrap();

    let new_err = env
        .get_static_field("java/lang/System", "err", "Ljava/io/PrintStream;")
        .unwrap()
        .l()
        .unwrap();
    assert_eq!(new_err.is_null(), false);
    let same_after = env.is_same_object(&new_err, &out).unwrap();
    assert_eq!(same_after, true);


    env.set_static_field(
        "java/lang/System",
        ("java/lang/System", "err", "Ljava/io/PrintStream;"),
        JValue::Object(&err_original),
    )
    .unwrap();

    let restored = env
        .get_static_field("java/lang/System", "err", "Ljava/io/PrintStream;")
        .unwrap()
        .l()
        .unwrap();
    let restored_matches = env.is_same_object(&restored, &err_original).unwrap();
    assert_eq!(restored_matches, true);
    let restored_is_not_out = env.is_same_object(&restored, &out).unwrap();
    assert_eq!(restored_is_not_out, false);


    let bad = env.set_static_field(
        "java/lang/System",
        ("java/lang/System", "nonexistent_xyz_field", "Ljava/io/PrintStream;"),
        JValue::Object(&out),
    );
    assert!(bad.is_err());
    if env.exception_check().unwrap_or(false) {
        env.exception_clear().unwrap();
    }
}

#[test]
fn test_rust_field_lifecycle() {
    let mut env = jvm().attach_current_thread().unwrap();


    let obj = env
        .new_object("java/util/concurrent/atomic/AtomicLong", "()V", &[])
        .unwrap();
    assert_eq!(obj.is_null(), false);


    let pre = env.get_field(&obj, "value", "J").unwrap();
    assert_eq!(pre.j().unwrap(), 0i64);


    let payload = String::from("rust-in-jvm");
    unsafe { env.set_rust_field(&obj, "value", payload).unwrap(); }


    let after_set = env.get_field(&obj, "value", "J").unwrap();
    assert_ne!(after_set.j().unwrap(), 0i64);


    {
        let guard = unsafe { env.get_rust_field::<_, _, String>(&obj, "value").unwrap() };
        assert_eq!(&*guard, "rust-in-jvm");
        assert_eq!(guard.len(), 11);
    }


    {
        let guard2 = unsafe { env.get_rust_field::<_, _, String>(&obj, "value").unwrap() };
        assert_eq!(&*guard2, "rust-in-jvm");
        assert_eq!(guard2.chars().count(), 11);
    }


    let taken: String = unsafe { env.take_rust_field(&obj, "value").unwrap() };
    assert_eq!(taken, "rust-in-jvm");
    assert_eq!(taken.len(), 11);


    let post = env.get_field(&obj, "value", "J").unwrap();
    assert_eq!(post.j().unwrap(), 0i64);


    {
        let gone = unsafe { env.get_rust_field::<_, _, String>(&obj, "value") };
        assert!(gone.is_err());
    }


    unsafe { env.set_rust_field(&obj, "value", 0xDEAD_BEEFu64).unwrap(); }
    {
        let g = unsafe { env.get_rust_field::<_, _, u64>(&obj, "value").unwrap() };
        assert_eq!(*g, 0xDEAD_BEEFu64);
    }

    let back: u64 = unsafe { env.take_rust_field(&obj, "value").unwrap() };
    assert_eq!(back, 0xDEAD_BEEFu64);

    let final_v = env.get_field(&obj, "value", "J").unwrap();
    assert_eq!(final_v.j().unwrap(), 0i64);
}

#[test]
fn test_lock_obj_monitor() {
    let mut env = jvm().attach_current_thread().unwrap();

    let obj1 = env.new_object("java/lang/Object", "()V", &[]).unwrap();
    let obj2 = env.new_object("java/lang/Object", "()V", &[]).unwrap();
    assert_eq!(obj1.is_null(), false);
    assert_eq!(obj2.is_null(), false);

    let same01 = env.is_same_object(&obj1, &obj2).unwrap();
    assert_eq!(same01, false);


    {
        let _g1 = env.lock_obj(&obj1).unwrap();

        let _g2 = env.lock_obj(&obj2).unwrap();
    }


    {
        let _g = env.lock_obj(&obj1).unwrap();
    }


    let s = env.new_string("locked").unwrap();
    {
        let _gs = env.lock_obj(&s).unwrap();
    }


    let mut acquired = 0;
    for _ in 0..16 {
        let _g = env.lock_obj(&obj1).unwrap();
        acquired += 1;
    }
    assert_eq!(acquired, 16);


    let mut depth = 0;
    {
        let _g1 = env.lock_obj(&obj1).unwrap();
        depth += 1;
        {
            let _g2 = env.lock_obj(&obj1).unwrap();
            depth += 1;
            {
                let _g3 = env.lock_obj(&obj1).unwrap();
                depth += 1;
            }
        }
    }
    assert_eq!(depth, 3);


    let _final_g = env.lock_obj(&obj2).unwrap();
    let final_same = env.is_same_object(&obj1, &obj1).unwrap();
    assert_eq!(final_same, true);
}

#[test]
fn test_ensure_local_capacity_and_refs() {
    let mut env = jvm().attach_current_thread().unwrap();


    env.ensure_local_capacity(1).unwrap();
    env.ensure_local_capacity(8).unwrap();
    env.ensure_local_capacity(64).unwrap();
    env.ensure_local_capacity(512).unwrap();
    env.ensure_local_capacity(0).unwrap();


    env.ensure_local_capacity(200).unwrap();

    let mut objs = Vec::with_capacity(128);
    for i in 0..128 {
        let s = env.new_string(format!("item_{:03}", i)).unwrap();
        objs.push(s);
    }
    assert_eq!(objs.len(), 128);


    {
        let s = env.get_string(&objs[0]).unwrap();
        let r = s.to_str().unwrap();
        assert_eq!(r, "item_000");
    }
    {
        let s = env.get_string(&objs[64]).unwrap();
        let r = s.to_str().unwrap();
        assert_eq!(r, "item_064");
    }
    {
        let s = env.get_string(&objs[127]).unwrap();
        let r = s.to_str().unwrap();
        assert_eq!(r, "item_127");
    }

    let count_before_clear = objs.len();
    objs.clear();
    assert_eq!(count_before_clear, 128);
    assert_eq!(objs.len(), 0);


    env.ensure_local_capacity(32).unwrap();

    let x = env.new_string("after-free").unwrap();
    let xs = env.get_string(&x).unwrap();
    let xs_str = xs.to_str().unwrap();
    assert_eq!(xs_str, "after-free");
    assert_eq!(xs_str.len(), 10);
}

extern "system" fn dummy_native_fn() {}

#[test]
fn test_register_unregister_native_methods() {
    let mut env = jvm().attach_current_thread().unwrap();

    let class = env.find_class("java/lang/String").unwrap();
    assert_eq!(class.is_null(), false);


    let methods = [NativeMethod {
        name: "intern".into(),
        sig: "()Ljava/lang/String;".into(),
        fn_ptr: dummy_native_fn as *mut c_void,
    }];

    let reg_result = env.register_native_methods("java/lang/String", &methods);
    assert!(reg_result.is_ok());


    let unreg_result = env.unregister_native_methods("java/lang/String");
    assert!(unreg_result.is_ok());
}