use jni::objects::{JObject, JValueGen, JValueOwned};
use jni::signature::Primitive;
use jni::sys::{
    jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort, JNI_FALSE, JNI_TRUE,
};

#[test]
fn primitive_jvalues_report_types_and_round_trip_through_jni_unions() {
    let boolean_value = JValueGen::<JObject<'static>>::Bool(JNI_TRUE);
    assert_eq!(JValueGen::type_name(&boolean_value), "bool");
    assert_eq!(
        JValueGen::primitive_type(&boolean_value),
        Some(Primitive::Boolean)
    );
    assert_eq!(
        JValueGen::z(boolean_value).expect("Bool.z should succeed"),
        true
    );

    let byte_value = JValueGen::<JObject<'static>>::Byte(-12 as jbyte);
    assert_eq!(JValueGen::type_name(&byte_value), "byte");
    assert_eq!(
        JValueGen::primitive_type(&byte_value),
        Some(Primitive::Byte)
    );
    assert_eq!(unsafe { JValueGen::as_jni(&byte_value).b }, -12 as jbyte);
    assert_eq!(
        JValueGen::b(byte_value).expect("Byte.b should succeed"),
        -12 as jbyte
    );

    let char_value = JValueGen::<JObject<'static>>::Char('λ' as jchar);
    assert_eq!(JValueGen::type_name(&char_value), "char");
    assert_eq!(
        JValueGen::primitive_type(&char_value),
        Some(Primitive::Char)
    );
    assert_eq!(
        JValueGen::c(char_value).expect("Char.c should succeed"),
        'λ' as jchar
    );

    let short_value = JValueGen::<JObject<'static>>::Short(-1234 as jshort);
    assert_eq!(JValueGen::type_name(&short_value), "short");
    assert_eq!(
        JValueGen::primitive_type(&short_value),
        Some(Primitive::Short)
    );
    assert_eq!(
        JValueGen::s(short_value).expect("Short.s should succeed"),
        -1234 as jshort
    );

    let int_value = JValueGen::<JObject<'static>>::Int(123_456 as jint);
    assert_eq!(JValueGen::type_name(&int_value), "int");
    assert_eq!(JValueGen::primitive_type(&int_value), Some(Primitive::Int));
    assert_eq!(unsafe { JValueGen::as_jni(&int_value).i }, 123_456 as jint);
    assert_eq!(
        JValueGen::i(int_value).expect("Int.i should succeed"),
        123_456 as jint
    );

    let long_value = JValueGen::<JObject<'static>>::Long(-9_876_543_210_i64 as jlong);
    assert_eq!(JValueGen::type_name(&long_value), "long");
    assert_eq!(
        JValueGen::primitive_type(&long_value),
        Some(Primitive::Long)
    );
    assert_eq!(
        JValueGen::j(long_value).expect("Long.j should succeed"),
        -9_876_543_210_i64 as jlong
    );

    let float_value = JValueGen::<JObject<'static>>::Float(3.5 as jfloat);
    assert_eq!(JValueGen::type_name(&float_value), "float");
    assert_eq!(
        JValueGen::primitive_type(&float_value),
        Some(Primitive::Float)
    );
    assert_eq!(
        JValueGen::f(float_value).expect("Float.f should succeed"),
        3.5 as jfloat
    );

    let double_value = JValueGen::<JObject<'static>>::Double(-7.25 as jdouble);
    assert_eq!(JValueGen::type_name(&double_value), "double");
    assert_eq!(
        JValueGen::primitive_type(&double_value),
        Some(Primitive::Double)
    );
    assert_eq!(
        JValueGen::d(double_value).expect("Double.d should succeed"),
        -7.25 as jdouble
    );

    let void_value = JValueGen::<JObject<'static>>::Void;
    assert_eq!(JValueGen::type_name(&void_value), "void");
    assert_eq!(
        JValueGen::primitive_type(&void_value),
        Some(Primitive::Void)
    );
    JValueGen::v(void_value).expect("Void.v should succeed");
}

#[test]
fn as_jni_preserves_payloads() {
    let int_value = JValueGen::<JObject<'static>>::Int(jint::MIN + 17);
    let int_union = JValueGen::as_jni(&int_value);
    assert_eq!(unsafe { int_union.i }, jint::MIN + 17);

    let false_value = JValueGen::<JObject<'static>>::Bool(JNI_FALSE as jboolean);
    let false_union = JValueGen::as_jni(&false_value);
    assert_eq!(unsafe { false_union.z }, JNI_FALSE as jboolean);

    let double_value = JValueGen::<JObject<'static>>::Double(42.125 as jdouble);
    let double_union = JValueGen::as_jni(&double_value);
    assert_eq!(unsafe { double_union.d }, 42.125 as jdouble);

    let null_object = JObject::null();
    let raw_null = JObject::as_raw(&null_object);
    let object_value = JValueGen::Object(null_object);
    let object_union = JValueGen::as_jni(&object_value);
    assert_eq!(unsafe { object_union.l }, raw_null);
    assert!(unsafe { object_union.l }.is_null());
}

#[test]
fn object_values_can_be_borrowed_and_extracted_without_losing_identity() {
    let owned_null = JObject::null();
    let owned_raw = JObject::as_raw(&owned_null);
    let owned_value: JValueOwned<'static> = JValueGen::Object(owned_null);

    assert_eq!(JValueGen::type_name(&owned_value), "object");
    assert_eq!(JValueGen::primitive_type(&owned_value), None);

    let borrowed_value = JValueGen::borrow(&owned_value);
    assert_eq!(JValueGen::type_name(&borrowed_value), "object");
    assert_eq!(JValueGen::primitive_type(&borrowed_value), None);

    let borrowed_object = JValueGen::l(borrowed_value).expect("borrowed Object.l should succeed");
    assert_eq!(JObject::as_raw(borrowed_object), owned_raw);
    assert!(JObject::as_raw(borrowed_object).is_null());

    let extracted_object = JValueGen::l(owned_value).expect("owned Object.l should succeed");
    assert_eq!(JObject::as_raw(&extracted_object), owned_raw);
}

#[test]
fn wrong_accessor_calls_return_errors_instead_of_reinterpreting_values() {
    let int_value = JValueGen::<JObject<'static>>::Int(99);
    assert!(
        JValueGen::z(int_value).is_err(),
        "z on an int value should reject the type mismatch"
    );

    let bool_value = JValueGen::<JObject<'static>>::Bool(JNI_TRUE);
    assert!(
        JValueGen::i(bool_value).is_err(),
        "i on a boolean value should reject the type mismatch"
    );

    let object_value = JValueGen::Object(JObject::null());
    assert!(
        JValueGen::v(object_value).is_err(),
        "v on an object value should reject the type mismatch"
    );

    let void_value = JValueGen::<JObject<'static>>::Void;
    assert!(
        JValueGen::l(void_value).is_err(),
        "l on a void value should reject the type mismatch"
    );
}