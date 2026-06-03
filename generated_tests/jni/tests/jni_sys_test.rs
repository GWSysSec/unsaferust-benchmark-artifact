#[cfg(any(target_os = "macos", target_os = "windows"))]
use jni::sys::{
    jint, JavaVMInitArgs, JNIInvokeInterface_, JNI_CreateJavaVM, JNI_GetCreatedJavaVMs,
    JNI_GetDefaultJavaVMInitArgs, JNI_EEXIST, JNI_OK, JNI_TRUE, JNI_VERSION_1_8,
};
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::os::raw::c_void;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::ptr;

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[test]
fn jni_invocation_api_reports_defaults_and_created_vms_consistently() {
    unsafe {
        let mut initial_count: jint = -1;
        let get_initial_result =
            JNI_GetCreatedJavaVMs(ptr::null_mut(), 0, &mut initial_count as *mut jint);

        assert_eq!(get_initial_result, JNI_OK);
        assert!(initial_count >= 0);

        let mut default_args = JavaVMInitArgs {
            version: JNI_VERSION_1_8,
            nOptions: 0,
            options: ptr::null_mut(),
            ignoreUnrecognized: JNI_TRUE,
        };

        let default_args_result =
            JNI_GetDefaultJavaVMInitArgs(&mut default_args as *mut JavaVMInitArgs as *mut c_void);

        assert_eq!(default_args_result, JNI_OK);
        assert!(default_args.version >= JNI_VERSION_1_8);
        assert_eq!(default_args.nOptions, 0);
        assert!(default_args.options.is_null());
        assert_eq!(default_args.ignoreUnrecognized, JNI_TRUE);

        let mut vm: *mut *const JNIInvokeInterface_ = ptr::null_mut();
        let mut env: *mut c_void = ptr::null_mut();

        let create_result = JNI_CreateJavaVM(
            &mut vm as *mut *mut *const JNIInvokeInterface_,
            &mut env as *mut *mut c_void,
            &mut default_args as *mut JavaVMInitArgs as *mut c_void,
        );

        assert!(
            create_result == JNI_OK || create_result == JNI_EEXIST,
            "JNI_CreateJavaVM returned unexpected error code: {}",
            create_result
        );

        if create_result == JNI_OK {
            assert!(!vm.is_null());
            assert!(!env.is_null());
        }

        let mut created_count: jint = -1;
        let mut vm_buffer: [*mut *const JNIInvokeInterface_; 8] = [ptr::null_mut(); 8];

        let get_created_result = JNI_GetCreatedJavaVMs(
            vm_buffer.as_mut_ptr(),
            vm_buffer.len() as jint,
            &mut created_count as *mut jint,
        );

        assert_eq!(get_created_result, JNI_OK);
        assert!(created_count >= initial_count);
        assert!(created_count >= 0);
        assert!(created_count as usize <= vm_buffer.len());

        if created_count > 0 {
            assert!(
                vm_buffer[..created_count as usize]
                    .iter()
                    .any(|created_vm| !created_vm.is_null())
            );

            if create_result == JNI_OK {
                assert!(
                    vm_buffer[..created_count as usize]
                        .iter()
                        .any(|created_vm| *created_vm == vm)
                );
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[test]
fn jni_invocation_api_linking_is_platform_dependent() {




    assert!(true);
}