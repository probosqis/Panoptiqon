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

use crate::cache::Cache;

pub trait ConvertJava
   where Self: Sized
{
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local>;
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Self;
}

/// RepositoryCache<T>
impl<T> ConvertJava for Cache<T>
   where T: ConvertJava
{
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      self.create_jvm_instance(env)
   }

   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Self {
      unsafe {
         Self::from_jvm_instance(env, &java_object)
      }
   }
}

/// T?
impl<T> ConvertJava for Option<T>
   where T: ConvertJava
{
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      match self {
         Some(t) => t.clone_into_java(env),
         None => JObject::null()
      }
   }

   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Self {
      if java_object.is_null() {
         None
      } else {
         Some(T::clone_from_java(env, java_object))
      }
   }
}

/// java.lang.Boolean
impl ConvertJava for bool {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Boolean", "valueOf", "(Z)Ljava/lang/Boolean;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }

   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Self {
      env.call_method(&java_object, "booleanValue", "()Z", &[]).unwrap().z().unwrap()
   }
}

/// java.lang.Byte
impl ConvertJava for i8 {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Byte", "valueOf", "(B)Ljava/lang/Byte;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }

   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Self {
      env.call_method(&java_object, "byteValue", "()B", &[]).unwrap().b().unwrap()
   }
}

/// java.lang.Short
impl ConvertJava for i16 {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Short", "valueOf", "(S)Ljava/lang/Short;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }

   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Self {
      env.call_method(&java_object, "shortValue", "()S", &[]).unwrap().s().unwrap()
   }
}

/// java.lang.Integer
impl ConvertJava for i32 {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
            "java/lang/Integer", "valueOf", "(I)Ljava/lang/Integer;", &[(*self).into()]
         ).unwrap().l().unwrap()
   }

   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Self {
      env.call_method(&java_object, "intValue", "()I", &[]).unwrap().i().unwrap()
   }
}

/// java.lang.Long
impl ConvertJava for i64 {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Long", "valueOf", "(J)Ljava/lang/Long;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }

   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Self {
      env.call_method(&java_object, "longValue", "()J", &[]).unwrap().j().unwrap()
   }
}

/// java.lang.Float
impl ConvertJava for f32 {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Float", "valueOf", "(F)Ljava/lang/Float;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }

   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Self {
      env.call_method(&java_object, "floatValue", "()F", &[]).unwrap().f().unwrap()
   }
}

/// java.lang.Double
impl ConvertJava for f64 {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Double", "valueOf", "(D)Ljava/lang/Double;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }

   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Self {
      env.call_method(&java_object, "doubleValue", "()D", &[]).unwrap().d().unwrap()
   }
}

/// java.lang.String
impl ConvertJava for String {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.new_string(&self).unwrap().into()
   }

   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Self {
      env.get_string(java_object.into()).unwrap().into()
   }
}

/// kotlin.Unit
impl ConvertJava for () {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.get_static_field("kotlin/Unit", "INSTANCE", "Lkotlin/Unit;")
         .unwrap().l().unwrap()
   }

   fn clone_from_java(_env: &mut JNIEnv, _java_object: &JObject) -> Self {
      ()
   }
}

/// kotlin.Pair
impl<A, B> ConvertJava for (A, B)
   where A: ConvertJava, B: ConvertJava
{
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      let first  = self.0.clone_into_java(env);
      let second = self.1.clone_into_java(env);

      env.new_object(
         "kotlin/Pair", "(Ljava/lang/Object;Ljava/lang/Object;)V",
         &[(&first).into(), (&second).into()]
      ).unwrap()
   }

   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Self {
      let first = env
         .call_method(java_object, "getFirst", "()Ljava/lang/Object;", &[])
         .unwrap().l().unwrap();
      let second = env
         .call_method(java_object, "getSecond", "()Ljava/lang/Object;", &[])
         .unwrap().l().unwrap();

      (A::clone_from_java(env, &first), B::clone_from_java(env, &second))
   }
}

/// kotlin.Triple
impl<A, B, C> ConvertJava for (A, B, C)
   where A: ConvertJava, B: ConvertJava, C: ConvertJava
{
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      let first  = self.0.clone_into_java(env);
      let second = self.1.clone_into_java(env);
      let third  = self.2.clone_into_java(env);

      env.new_object(
         "kotlin/Triple", "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)V",
         &[(&first).into(), (&second).into(), (&third).into()]
      ).unwrap()
   }

   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Self {
      let first = env
         .call_method(java_object, "getFirst", "()Ljava/lang/Object;", &[])
         .unwrap().l().unwrap();
      let second = env
         .call_method(java_object, "getSecond", "()Ljava/lang/Object;", &[])
         .unwrap().l().unwrap();
      let third = env
         .call_method(java_object, "getThird", "()Ljava/lang/Object;", &[])
         .unwrap().l().unwrap();

      (
         A::clone_from_java(env, &first),
         B::clone_from_java(env, &second),
         C::clone_from_java(env, &third)
      )
   }
}
