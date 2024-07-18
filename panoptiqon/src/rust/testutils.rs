/*
 * Copyright 2024 wcaokaze
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use std::ptr::null_mut;

use jni::JavaVM;
use jni::sys::{JNI_GetCreatedJavaVMs, JNI_OK, jsize};

#[cfg(feature="jni-test")]
pub(crate) fn get_vm() -> JavaVM {
   let mut buf: [*mut jni::sys::JavaVM; 4] = [null_mut(); 4];
   let mut vm_count = 0;

   let result_code = unsafe {
      JNI_GetCreatedJavaVMs(
         &mut buf as *mut _,
         buf.len() as jsize,
         &mut vm_count as *mut _
      )
   };

   if result_code != JNI_OK { panic!(); }
   if vm_count != 1 { panic!(); }

   unsafe {
      JavaVM::from_raw(buf[0]).unwrap()
   }
}
