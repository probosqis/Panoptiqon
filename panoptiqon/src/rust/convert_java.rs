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
#![cfg(feature="jvm")]

use jni::JNIEnv;
use jni::objects::JObject;

pub trait ConvertJava
   where Self: Sized
{
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local>;
   fn clone_from_java(env: &mut JNIEnv, java_object: JObject) -> Self;
}

/// java.lang.Integer
impl ConvertJava for i32 {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
            "java/lang/Integer", "valueOf", "(I)Ljava/lang/Integer;", &[(*self).into()]
         ).unwrap().l().unwrap()
   }

   fn clone_from_java(env: &mut JNIEnv, java_object: JObject) -> Self {
      env.call_method(&java_object, "intValue", "()I", &[]).unwrap().i().unwrap()
   }
}
