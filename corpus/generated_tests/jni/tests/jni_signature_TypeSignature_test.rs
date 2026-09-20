use jni::signature::{JavaType, Primitive, ReturnType, TypeSignature};

#[test]
fn parses_method_signature_with_primitives_objects_arrays_and_void_return() {
    let sig = TypeSignature::from_str("(ILjava/lang/String;[B[[D)V")
        .expect("valid JNI method signature should parse");

    assert_eq!(sig.args.len(), 4);
    assert!(matches!(&sig.args[0], JavaType::Primitive(Primitive::Int)));
    assert!(matches!(
        &sig.args[1],
        JavaType::Object(class_name) if class_name == "java/lang/String"
    ));

    match &sig.args[2] {
        JavaType::Array(inner) => {
            assert!(matches!(
                inner.as_ref(),
                JavaType::Primitive(Primitive::Byte)
            ));
        }
        other => panic!("expected byte array argument, got {:?}", other),
    }

    match &sig.args[3] {
        JavaType::Array(outer) => match outer.as_ref() {
            JavaType::Array(inner) => {
                assert!(matches!(
                    inner.as_ref(),
                    JavaType::Primitive(Primitive::Double)
                ));
            }
            other => panic!("expected nested double array argument, got {:?}", other),
        },
        other => panic!("expected nested double array argument, got {:?}", other),
    }

    assert!(matches!(sig.ret, ReturnType::Primitive(Primitive::Void)));
}

#[test]
fn parses_signature_with_array_argument_and_object_return() {
    let sig = TypeSignature::from_str("([Ljava/lang/String;[[I)Ljava/lang/Object;")
        .expect("valid object-returning JNI method signature should parse");

    assert_eq!(sig.args.len(), 2);

    match &sig.args[0] {
        JavaType::Array(inner) => {
            assert!(matches!(
                inner.as_ref(),
                JavaType::Object(class_name) if class_name == "java/lang/String"
            ));
        }
        other => panic!("expected string array argument, got {:?}", other),
    }

    match &sig.args[1] {
        JavaType::Array(outer) => match outer.as_ref() {
            JavaType::Array(inner) => {
                assert!(matches!(
                    inner.as_ref(),
                    JavaType::Primitive(Primitive::Int)
                ));
            }
            other => panic!("expected nested int array argument, got {:?}", other),
        },
        other => panic!("expected nested int array argument, got {:?}", other),
    }

    assert!(matches!(sig.ret, ReturnType::Object));
}

#[test]
fn parses_no_argument_primitive_return_signature() {
    let sig = TypeSignature::from_str("()Z")
        .expect("valid no-argument boolean-returning method signature should parse");

    assert!(sig.args.is_empty());
    assert!(matches!(
        sig.ret,
        ReturnType::Primitive(Primitive::Boolean)
    ));
}

#[test]
fn rejects_malformed_signatures_without_partially_accepting_them() {
    let invalid_signatures = [
        "",
        "I",
        "(",
        ")V",
        "(I",
        "(I)",
        "(Q)V",
        "([)V",
        "(I)Q",
    ];

    for invalid in invalid_signatures {
        assert!(
            TypeSignature::from_str(invalid).is_err(),
            "signature {:?} should be rejected",
            invalid
        );
    }
}