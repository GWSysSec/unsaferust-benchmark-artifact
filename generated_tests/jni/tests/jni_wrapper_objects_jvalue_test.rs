use jni::objects::{JObject, JValueGen, JValueOwned};
use jni::signature::Primitive;
use jni::sys::{
    jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort, JNI_FALSE, JNI_TRUE,
};

#[test]
fn primitive_jvalues_expose_expected_type_metadata_and_jni_union_fields() {
    let bool_value = JValueGen::<JObject<'static>>::Bool(JNI_TRUE as jboolean);
    assert_eq!(jni::objects::JValueGen::type_name(&bool_value), "bool");
    assert_eq!(
        jni::objects::JValueGen::primitive_type(&bool_value),
        Some(Primitive::Boolean)
    );
    assert_eq!(unsafe { jni::objects::JValueGen::as_jni(&bool_value).z }, JNI_TRUE);
    assert_eq!(
        jni::objects::JValueGen::z(bool_value).expect("Bool.z should decode JNI_TRUE"),
        true
    );

    let false_value = JValueGen::<JObject<'static>>::Bool(JNI_FALSE as jboolean);
    assert_eq!(
        jni::objects::JValueGen::z(false_value).expect("Bool.z should decode JNI_FALSE"),
        false
    );

    let byte_value = JValueGen::<JObject<'static>>::Byte(-7 as jbyte);
    assert_eq!(jni::objects::JValueGen::type_name(&byte_value), "byte");
    assert_eq!(
        jni::objects::JValueGen::primitive_type(&byte_value),
        Some(Primitive::Byte)
    );
    assert_eq!(unsafe { jni::objects::JValueGen::as_jni(&byte_value).b }, -7 as jbyte);
    assert_eq!(
        jni::objects::JValueGen::b(byte_value).expect("Byte.b should return the original byte"),
        -7 as jbyte
    );

    let char_value = JValueGen::<JObject<'static>>::Char(0x03BB as jchar);
    assert_eq!(jni::objects::JValueGen::type_name(&char_value), "char");
    assert_eq!(
        jni::objects::JValueGen::primitive_type(&char_value),
        Some(Primitive::Char)
    );
    assert_eq!(unsafe { jni::objects::JValueGen::as_jni(&char_value).c }, 0x03BB as jchar);
    assert_eq!(
        jni::objects::JValueGen::c(char_value).expect("Char.c should return the original char"),
        0x03BB as jchar
    );

    let short_value = JValueGen::<JObject<'static>>::Short(-1234 as jshort);
    assert_eq!(jni::objects::JValueGen::type_name(&short_value), "short");
    assert_eq!(
        jni::objects::JValueGen::primitive_type(&short_value),
        Some(Primitive::Short)
    );
    assert_eq!(unsafe { jni::objects::JValueGen::as_jni(&short_value).s }, -1234 as jshort);
    assert_eq!(
        jni::objects::JValueGen::s(short_value).expect("Short.s should return the original short"),
        -1234 as jshort
    );

    let int_value = JValueGen::<JObject<'static>>::Int(123_456 as jint);
    assert_eq!(jni::objects::JValueGen::type_name(&int_value), "int");
    assert_eq!(
        jni::objects::JValueGen::primitive_type(&int_value),
        Some(Primitive::Int)
    );
    assert_eq!(unsafe { jni::objects::JValueGen::as_jni(&int_value).i }, 123_456 as jint);
    assert_eq!(
        jni::objects::JValueGen::i(int_value).expect("Int.i should return the original int"),
        123_456 as jint
    );

    let long_value = JValueGen::<JObject<'static>>::Long(-9_876_543_210_i64 as jlong);
    assert_eq!(jni::objects::JValueGen::type_name(&long_value), "long");
    assert_eq!(
        jni::objects::JValueGen::primitive_type(&long_value),
        Some(Primitive::Long)
    );
    assert_eq!(
        unsafe { jni::objects::JValueGen::as_jni(&long_value).j },
        -9_876_543_210_i64 as jlong
    );
    assert_eq!(
        jni::objects::JValueGen::j(long_value).expect("Long.j should return the original long"),
        -9_876_543_210_i64 as jlong
    );

    let float_value = JValueGen::<JObject<'static>>::Float(12.5 as jfloat);
    assert_eq!(jni::objects::JValueGen::type_name(&float_value), "float");
    assert_eq!(
        jni::objects::JValueGen::primitive_type(&float_value),
        Some(Primitive::Float)
    );
    assert_eq!(unsafe { jni::objects::JValueGen::as_jni(&float_value).f }, 12.5 as jfloat);
    assert_eq!(
        jni::objects::JValueGen::f(float_value).expect("Float.f should return the original float"),
        12.5 as jfloat
    );

    let double_value = JValueGen::<JObject<'static>>::Double(-0.25 as jdouble);
    assert_eq!(jni::objects::JValueGen::type_name(&double_value), "double");
    assert_eq!(
        jni::objects::JValueGen::primitive_type(&double_value),
        Some(Primitive::Double)
    );
    assert_eq!(
        unsafe { jni::objects::JValueGen::as_jni(&double_value).d },
        -0.25 as jdouble
    );
    assert_eq!(
        jni::objects::JValueGen::d(double_value).expect("Double.d should return the original double"),
        -0.25 as jdouble
    );
}

#[test]
fn object_and_void_jvalues_have_no_primitive_type_and_round_trip_as_expected() {
    let null_object = JObject::null();
    let object_value = JValueGen::Object(null_object);

    assert_eq!(jni::objects::JValueGen::type_name(&object_value), "object");
    assert_eq!(jni::objects::JValueGen::primitive_type(&object_value), None);
    assert!(
        unsafe { jni::objects::JValueGen::as_jni(&object_value).l }.is_null(),
        "Object(null) should expose a null jobject through as_jni"
    );

    let raw_from_to_jni = jni::objects::JValueGen::to_jni(object_value);
    assert!(
        unsafe { raw_from_to_jni.l }.is_null(),
        "Object(null) should remain null after consuming to_jni"
    );

    let extracted_object = jni::objects::JValueGen::l(JValueGen::Object(JObject::null()))
        .expect("Object.l should return the wrapped object");
    assert!(
        JObject::as_raw(&extracted_object).is_null(),
        "Object.l should preserve the wrapped null object"
    );

    let void_value = JValueGen::<JObject<'static>>::Void;
    assert_eq!(jni::objects::JValueGen::type_name(&void_value), "void");
    assert_eq!(jni::objects::JValueGen::primitive_type(&void_value), Some(Primitive::Void));
    assert!(
        jni::objects::JValueGen::v(void_value).is_ok(),
        "Void.v should succeed for the Void variant"
    );
}

#[test]
fn borrowed_jvalue_keeps_object_identity_and_allows_read_only_conversion() {
    let owned: JValueOwned<'static> = JValueGen::Object(JObject::null());
    let borrowed = jni::objects::JValueGen::borrow(&owned);

    assert_eq!(jni::objects::JValueGen::type_name(&borrowed), "object");
    assert_eq!(jni::objects::JValueGen::primitive_type(&borrowed), None);
    assert!(
        unsafe { jni::objects::JValueGen::as_jni(&borrowed).l }.is_null(),
        "Borrowed view of Object(null) should expose the same null jobject"
    );

    let borrowed_object_ref = jni::objects::JValueGen::l(borrowed)
        .expect("Borrowed object value should extract as an object reference");
    assert!(
        JObject::as_raw(borrowed_object_ref).is_null(),
        "Borrowed object reference should point to the original null object"
    );
}

#[test]
fn accessors_reject_mismatched_jvalue_variants() {
    let int_value = JValueGen::<JObject<'static>>::Int(42 as jint);
    assert!(
        jni::objects::JValueGen::z(int_value).is_err(),
        "Bool accessor should reject an Int value"
    );

    let object_value = JValueGen::Object(JObject::null());
    assert!(
        jni::objects::JValueGen::i(object_value).is_err(),
        "Int accessor should reject an Object value"
    );

    let void_value = JValueGen::<JObject<'static>>::Void;
    assert!(
        jni::objects::JValueGen::l(void_value).is_err(),
        "Object accessor should reject a Void value"
    );

    let double_value = JValueGen::<JObject<'static>>::Double(3.0 as jdouble);
    assert!(
        jni::objects::JValueGen::v(double_value).is_err(),
        "Void accessor should reject a Double value"
    );
}