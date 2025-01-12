/*
 * Copyright 2024-2025 wcaokaze
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
use jni::signature::{Primitive, ReturnType};
use jni::sys::jvalue;
use crate::cache::Cache;

pub trait CloneIntoJava {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local>;
}

pub trait CloneFromJava
   where Self: Sized
{
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Self;
}

pub trait ConvertJava: CloneIntoJava + CloneFromJava {}
impl<AutoImpl> ConvertJava for AutoImpl where AutoImpl: CloneIntoJava + CloneFromJava {}

/// RepositoryCache<T>
impl<T> CloneIntoJava for Cache<T>
   // TODO: create_jvm_instanceがConvertJavaを要求しなくなったらCloneIntoJavaにする
   where T: ConvertJava
{
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      self.create_jvm_instance(env)
   }
}

impl<T> CloneFromJava for Cache<T>
   where T: ConvertJava
{
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Cache<T> {
      unsafe {
         Cache::<T>::from_jvm_instance(env, &java_object)
      }
   }
}

/// T
impl<T> CloneIntoJava for Box<T>
   where T: CloneIntoJava
{
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      self.as_ref().clone_into_java(env)
   }
}

impl<T> CloneFromJava for Box<T>
   where T: CloneFromJava
{
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Box<T> {
      Box::new(T::clone_from_java(env, java_object))
   }
}

/// java.util.ArrayList (=kotlin.collections.ArrayList)
impl<T> CloneIntoJava for Vec<T>
   where T: CloneIntoJava
{
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      let list = env.new_object("Ljava/util/ArrayList;", "()V", &[]).unwrap();

      let add_method_id = env
         .get_method_id("Ljava/util/ArrayList;", "add", "(Ljava/lang/Object;)Z")
         .unwrap();

      for element in self {
         let element = element.clone_into_java(env);
         unsafe {
            env.call_method_unchecked(
                  &list, add_method_id, ReturnType::Primitive(Primitive::Boolean),
                  &[jvalue { l: element.into_raw() }]
               )
               .unwrap();
         }
      }

      list
   }
}

impl<T> CloneFromJava for Vec<T>
   where T: CloneFromJava
{
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Vec<T> {
      let mut vec = Vec::new();

      let len = env.call_method(&java_object, "size", "()I", &[]).unwrap().i().unwrap();

      let get_method_id = env
         .get_method_id("Ljava/util/ArrayList;", "get", "(I)Ljava/lang/Object;")
         .unwrap();

      for index in 0..len {
         let element = unsafe {
            env.call_method_unchecked(
                  &java_object, get_method_id, ReturnType::Object,
                  &[jvalue { i: index }]
               )
               .unwrap().l().unwrap()
         };

         vec.push(T::clone_from_java(env, &element));
      }

      vec
   }
}

/// T?
impl<T> CloneIntoJava for Option<T>
   where T: CloneIntoJava
{
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      match self {
         Some(t) => t.clone_into_java(env),
         None => JObject::null()
      }
   }
}

impl<T> CloneFromJava for Option<T>
   where T: CloneFromJava
{
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> Option<T> {
      if java_object.is_null() {
         None
      } else {
         Some(T::clone_from_java(env, java_object))
      }
   }
}

/// java.lang.Boolean
impl CloneIntoJava for bool {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Boolean", "valueOf", "(Z)Ljava/lang/Boolean;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }
}

impl CloneFromJava for bool {
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> bool {
      env.call_method(&java_object, "booleanValue", "()Z", &[]).unwrap().z().unwrap()
   }
}

/// java.lang.Byte
impl CloneIntoJava for i8 {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Byte", "valueOf", "(B)Ljava/lang/Byte;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }
}

impl CloneFromJava for i8 {
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> i8 {
      env.call_method(&java_object, "byteValue", "()B", &[]).unwrap().b().unwrap()
   }
}

/// java.lang.Short
impl CloneIntoJava for i16 {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Short", "valueOf", "(S)Ljava/lang/Short;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }
}

impl CloneFromJava for i16 {
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> i16 {
      env.call_method(&java_object, "shortValue", "()S", &[]).unwrap().s().unwrap()
   }
}

/// java.lang.Integer
impl CloneIntoJava for i32 {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Integer", "valueOf", "(I)Ljava/lang/Integer;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }
}

impl CloneFromJava for i32 {
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> i32 {
      env.call_method(&java_object, "intValue", "()I", &[]).unwrap().i().unwrap()
   }
}

/// java.lang.Long
impl CloneIntoJava for i64 {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Long", "valueOf", "(J)Ljava/lang/Long;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }
}

impl CloneFromJava for i64 {
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> i64 {
      env.call_method(&java_object, "longValue", "()J", &[]).unwrap().j().unwrap()
   }
}

/// java.lang.Float
impl CloneIntoJava for f32 {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Float", "valueOf", "(F)Ljava/lang/Float;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }
}

impl CloneFromJava for f32 {
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> f32 {
      env.call_method(&java_object, "floatValue", "()F", &[]).unwrap().f().unwrap()
   }
}

/// java.lang.Double
impl CloneIntoJava for f64 {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Double", "valueOf", "(D)Ljava/lang/Double;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }
}

impl CloneFromJava for f64 {
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> f64 {
      env.call_method(&java_object, "doubleValue", "()D", &[]).unwrap().d().unwrap()
   }
}

/// java.lang.String
impl CloneIntoJava for String {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.new_string(&self).unwrap().into()
   }
}

impl CloneFromJava for String {
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> String {
      env.get_string(java_object.into()).unwrap().into()
   }
}

/// kotlin.Unit
impl CloneIntoJava for () {
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.get_static_field("kotlin/Unit", "INSTANCE", "Lkotlin/Unit;")
         .unwrap().l().unwrap()
   }
}

impl CloneFromJava for () {
   fn clone_from_java(_env: &mut JNIEnv, _java_object: &JObject) -> () {
      ()
   }
}

/// kotlin.Pair
impl<A, B> CloneIntoJava for (A, B)
   where A: CloneIntoJava, B: CloneIntoJava
{
   fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      let first  = self.0.clone_into_java(env);
      let second = self.1.clone_into_java(env);

      env.new_object(
         "kotlin/Pair", "(Ljava/lang/Object;Ljava/lang/Object;)V",
         &[(&first).into(), (&second).into()]
      ).unwrap()
   }
}

impl<A, B> CloneFromJava for (A, B)
   where A: CloneFromJava, B: CloneFromJava
{
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> (A, B) {
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
impl<A, B, C> CloneIntoJava for (A, B, C)
   where A: CloneIntoJava, B: CloneIntoJava, C: CloneIntoJava
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
}

impl<A, B, C> CloneFromJava for (A, B, C)
   where A: CloneFromJava, B: CloneFromJava, C: CloneFromJava
{
   fn clone_from_java(env: &mut JNIEnv, java_object: &JObject) -> (A, B, C) {
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
