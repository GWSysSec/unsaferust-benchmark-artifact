use jni::objects::{JObject, JValueGen};

#[test]
fn test_type_name_returns_distinct_strings() {
    let byte_val: JValueGen<JObject> = JValueGen::Byte(1i8);
    let char_val: JValueGen<JObject> = JValueGen::Char(2u16);
    let short_val: JValueGen<JObject> = JValueGen::Short(3i16);
    let int_val: JValueGen<JObject> = JValueGen::Int(4i32);
    let long_val: JValueGen<JObject> = JValueGen::Long(5i64);
    let float_val: JValueGen<JObject> = JValueGen::Float(6.0f32);
    let double_val: JValueGen<JObject> = JValueGen::Double(7.0f64);
    let bool_val: JValueGen<JObject> = JValueGen::Bool(1u8);
    let void_val: JValueGen<JObject> = JValueGen::Void;
    let null_obj = unsafe { JObject::from_raw(std::ptr::null_mut()) };
    let obj_val: JValueGen<JObject> = JValueGen::Object(null_obj);

    let names: [&'static str; 10] = [
        byte_val.type_name(),
        char_val.type_name(),
        short_val.type_name(),
        int_val.type_name(),
        long_val.type_name(),
        float_val.type_name(),
        double_val.type_name(),
        bool_val.type_name(),
        void_val.type_name(),
        obj_val.type_name(),
    ];

    for name in &names {
        assert!(!name.is_empty(), "type_name must not be empty");
    }

    assert_eq!(byte_val.type_name(), byte_val.type_name());
    assert_eq!(int_val.type_name(), int_val.type_name());


    for i in 0..names.len() {
        for j in (i + 1)..names.len() {
            assert_ne!(
                names[i], names[j],
                "type_name[{}]={:?} must differ from type_name[{}]={:?}",
                i, names[i], j, names[j]
            );
        }
    }
}

#[test]
fn test_correct_accessors_extract_values() {
    let byte_val: JValueGen<JObject> = JValueGen::Byte(-42i8);
    let char_val: JValueGen<JObject> = JValueGen::Char(0x4100u16);
    let short_val: JValueGen<JObject> = JValueGen::Short(-12345i16);
    let long_val: JValueGen<JObject> = JValueGen::Long(0x7FFF_FFFF_FFFF_FFFFi64);
    let float_val: JValueGen<JObject> = JValueGen::Float(1.5f32);
    let double_val: JValueGen<JObject> = JValueGen::Double(2.5f64);
    let void_val: JValueGen<JObject> = JValueGen::Void;

    assert_eq!(byte_val.b().unwrap(), -42i8);
    assert_eq!(char_val.c().unwrap(), 0x4100u16);
    assert_eq!(short_val.s().unwrap(), -12345i16);
    assert_eq!(long_val.j().unwrap(), 0x7FFF_FFFF_FFFF_FFFFi64);
    assert_eq!(float_val.f().unwrap(), 1.5f32);
    assert_eq!(double_val.d().unwrap(), 2.5f64);
    assert!(void_val.v().is_ok());

    let unit: () = JValueGen::<JObject>::Void.v().unwrap();
    assert_eq!(unit, ());
}

#[test]
fn test_wrong_accessor_returns_error() {
    let int_val: JValueGen<JObject> = JValueGen::Int(42i32);
    assert!(int_val.b().is_err());

    let byte_val: JValueGen<JObject> = JValueGen::Byte(1i8);
    assert!(byte_val.c().is_err());

    let char_val: JValueGen<JObject> = JValueGen::Char(1u16);
    assert!(char_val.d().is_err());

    let double_val: JValueGen<JObject> = JValueGen::Double(1.0f64);
    assert!(double_val.f().is_err());

    let float_val: JValueGen<JObject> = JValueGen::Float(1.0f32);
    assert!(float_val.j().is_err());

    let long_val: JValueGen<JObject> = JValueGen::Long(1i64);
    assert!(long_val.s().is_err());

    let short_val: JValueGen<JObject> = JValueGen::Short(1i16);
    assert!(short_val.v().is_err());

    let void_val: JValueGen<JObject> = JValueGen::Void;
    assert!(void_val.b().is_err());

    let null_obj = unsafe { JObject::from_raw(std::ptr::null_mut()) };
    let obj_val: JValueGen<JObject> = JValueGen::Object(null_obj);
    assert!(obj_val.d().is_err());
}

#[test]
fn test_to_jni_extracts_primitive_fields() {
    let byte_val: JValueGen<JObject> = JValueGen::Byte(-17i8);
    let char_val: JValueGen<JObject> = JValueGen::Char(0xABCDu16);
    let short_val: JValueGen<JObject> = JValueGen::Short(-9999i16);
    let int_val: JValueGen<JObject> = JValueGen::Int(123456i32);
    let long_val: JValueGen<JObject> = JValueGen::Long(-9876543210i64);
    let float_val: JValueGen<JObject> = JValueGen::Float(-0.5f32);
    let double_val: JValueGen<JObject> = JValueGen::Double(123.456f64);
    let bool_val: JValueGen<JObject> = JValueGen::Bool(1u8);


    unsafe {
        assert_eq!(byte_val.to_jni().b, -17i8);
        assert_eq!(char_val.to_jni().c, 0xABCDu16);
        assert_eq!(short_val.to_jni().s, -9999i16);
        assert_eq!(int_val.to_jni().i, 123456i32);
        assert_eq!(long_val.to_jni().j, -9876543210i64);
        assert_eq!(float_val.to_jni().f, -0.5f32);
        assert_eq!(double_val.to_jni().d, 123.456f64);
        assert_eq!(bool_val.to_jni().z, 1u8);
    }
}

#[test]
fn test_borrow_preserves_primitive_values() {
    let byte_val: JValueGen<JObject> = JValueGen::Byte(77i8);
    let char_val: JValueGen<JObject> = JValueGen::Char(999u16);
    let short_val: JValueGen<JObject> = JValueGen::Short(-888i16);
    let long_val: JValueGen<JObject> = JValueGen::Long(999_999i64);
    let float_val: JValueGen<JObject> = JValueGen::Float(42.5f32);
    let double_val: JValueGen<JObject> = JValueGen::Double(-42.5f64);
    let void_val: JValueGen<JObject> = JValueGen::Void;


    {
        let borrowed_b = byte_val.borrow();
        assert_eq!(borrowed_b.b().unwrap(), 77i8);
    }
    {
        let borrowed_c = char_val.borrow();
        assert_eq!(borrowed_c.c().unwrap(), 999u16);
    }
    {
        let borrowed_s = short_val.borrow();
        assert_eq!(borrowed_s.s().unwrap(), -888i16);
    }
    {
        let borrowed_j = long_val.borrow();
        assert_eq!(borrowed_j.j().unwrap(), 999_999i64);
    }
    {
        let borrowed_f = float_val.borrow();
        assert_eq!(borrowed_f.f().unwrap(), 42.5f32);
    }
    {
        let borrowed_d = double_val.borrow();
        assert_eq!(borrowed_d.d().unwrap(), -42.5f64);
    }
    {
        let borrowed_v = void_val.borrow();
        assert!(borrowed_v.v().is_ok());
    }


    assert_eq!(byte_val.b().unwrap(), 77i8);
    assert_eq!(char_val.c().unwrap(), 999u16);
}

#[test]
fn test_borrow_wrong_accessor_returns_error() {
    let int_val: JValueGen<JObject> = JValueGen::Int(100i32);
    let long_val: JValueGen<JObject> = JValueGen::Long(200i64);
    let float_val: JValueGen<JObject> = JValueGen::Float(300.0f32);
    let short_val: JValueGen<JObject> = JValueGen::Short(7i16);

    assert!(int_val.borrow().b().is_err());
    assert!(long_val.borrow().c().is_err());
    assert!(float_val.borrow().d().is_err());
    assert!(short_val.borrow().v().is_err());

    let null_obj = unsafe { JObject::from_raw(std::ptr::null_mut()) };
    let obj_val: JValueGen<JObject> = JValueGen::Object(null_obj);

    assert!(obj_val.borrow().b().is_err());
    assert!(obj_val.borrow().c().is_err());
    assert!(obj_val.borrow().s().is_err());
    assert!(obj_val.borrow().j().is_err());
    assert!(obj_val.borrow().f().is_err());
    assert!(obj_val.borrow().d().is_err());
    assert!(obj_val.borrow().v().is_err());
}

#[test]
fn test_boundary_values_roundtrip_through_to_jni_and_accessors() {

    let min_byte: JValueGen<JObject> = JValueGen::Byte(i8::MIN);
    let max_byte: JValueGen<JObject> = JValueGen::Byte(i8::MAX);
    assert_eq!(min_byte.b().unwrap(), i8::MIN);
    assert_eq!(max_byte.b().unwrap(), i8::MAX);

    let min_short: JValueGen<JObject> = JValueGen::Short(i16::MIN);
    let max_short: JValueGen<JObject> = JValueGen::Short(i16::MAX);
    assert_eq!(min_short.s().unwrap(), i16::MIN);
    assert_eq!(max_short.s().unwrap(), i16::MAX);

    let min_long: JValueGen<JObject> = JValueGen::Long(i64::MIN);
    let max_long: JValueGen<JObject> = JValueGen::Long(i64::MAX);
    assert_eq!(min_long.j().unwrap(), i64::MIN);
    assert_eq!(max_long.j().unwrap(), i64::MAX);

    let zero_char: JValueGen<JObject> = JValueGen::Char(0u16);
    let max_char: JValueGen<JObject> = JValueGen::Char(u16::MAX);
    assert_eq!(zero_char.c().unwrap(), 0u16);
    assert_eq!(max_char.c().unwrap(), u16::MAX);


    let neg_zero_f: JValueGen<JObject> = JValueGen::Float(-0.0f32);
    let inf_f: JValueGen<JObject> = JValueGen::Float(f32::INFINITY);
    let nan_d: JValueGen<JObject> = JValueGen::Double(f64::NAN);

    unsafe {
        let nz = neg_zero_f.to_jni().f;
        assert_eq!(nz, 0.0f32);
        assert!(nz.is_sign_negative());
        let inf = inf_f.to_jni().f;
        assert!(inf.is_infinite());
        assert!(inf.is_sign_positive());
        let n = nan_d.to_jni().d;
        assert!(n.is_nan());
    }
}