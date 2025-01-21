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
#![cfg(feature = "jvm")]

use std::sync::{Arc, RwLock};
use jni::JNIEnv;
use jni::objects::JObject;
use jni::sys::jvalue;
use crate::cache::{Cache, CacheContent};
use crate::unique_cache::{
   DynOneWayUniqueCache, DynTwoWayUniqueCache, JvmUniqueCacheRefs, UniqueCache,
};

type VTable = ();

/// [CloneIntoJni]を実装すれば自動的に実装される。
/// こちらを手で実装する必要はない
pub trait CloneIntoJavaHelper
   where Self: CacheContent + Sized
{
   #[allow(private_interfaces)]
   fn get_jvm_refs(env: &mut JNIEnv) -> JvmUniqueCacheRefs;

   fn get_unique_cache_address(
      unique_cache: Arc<RwLock<UniqueCache<Self>>>
   ) -> (*const RwLock<UniqueCache<Self>>, *const VTable);
}

impl<T> CloneIntoJavaHelper for T where T: CacheContent + CloneIntoJni
{
   #[allow(private_interfaces)]
   default fn get_jvm_refs(env: &mut JNIEnv) -> JvmUniqueCacheRefs {
      let class = env
         .find_class("com/wcaokaze/probosqis/panoptiqon/UniqueCache").unwrap();
      let class = env.new_global_ref(class).unwrap();
      let constructor_id = env
         .get_method_id(&class, "<init>", "(Ljava/lang/Object;JJ)V").unwrap();
      let update_method_id = env
         .get_method_id(&class, "updateStateFromNative", "(Ljava/lang/Object;)V").unwrap();

      let cache_class = env
         .find_class("com/wcaokaze/probosqis/panoptiqon/RepositoryCache").unwrap();
      let cache_class = env.new_global_ref(cache_class).unwrap();
      let cache_constructor_id = env
         .get_method_id(&cache_class, "<init>", "(Lcom/wcaokaze/probosqis/panoptiqon/UniqueCache;)V").unwrap();

      JvmUniqueCacheRefs {
         class, constructor_id, update_method_id, cache_class, cache_constructor_id
      }
   }

   default fn get_unique_cache_address(
      unique_cache: Arc<RwLock<UniqueCache<Self>>>
   ) -> (*const RwLock<UniqueCache<Self>>, *const VTable) {
      use std::mem;
      use std::ptr;

      let trait_obj: *const dyn DynOneWayUniqueCache = Arc::into_raw(unique_cache);
      let unique_cache_address = trait_obj as *const RwLock<UniqueCache<Self>>;

      let trait_object_metadata = ptr::metadata(trait_obj);
      let vtable_address = unsafe {
         mem::transmute::<_, *const ()>(trait_object_metadata)
      };

      (unique_cache_address, vtable_address)
   }
}

impl<T> CloneIntoJavaHelper for T where T: CacheContent + CloneIntoJni + CloneFromJni
{
   #[allow(private_interfaces)]
   fn get_jvm_refs(env: &mut JNIEnv) -> JvmUniqueCacheRefs {
      let class = env
         .find_class("com/wcaokaze/probosqis/panoptiqon/WritableUniqueCache").unwrap();
      let class = env.new_global_ref(class).unwrap();
      let constructor_id = env
         .get_method_id(&class, "<init>", "(Ljava/lang/Object;JJ)V").unwrap();
      let update_method_id = env
         .get_method_id(&class, "updateStateFromNative", "(Ljava/lang/Object;)V").unwrap();

      let cache_class = env
         .find_class("com/wcaokaze/probosqis/panoptiqon/WritableRepositoryCache").unwrap();
      let cache_class = env.new_global_ref(cache_class).unwrap();
      let cache_constructor_id = env
         .get_method_id(&cache_class, "<init>", "(Lcom/wcaokaze/probosqis/panoptiqon/WritableUniqueCache;)V").unwrap();

      JvmUniqueCacheRefs {
         class, constructor_id, update_method_id, cache_class, cache_constructor_id
      }
   }

   fn get_unique_cache_address(
      unique_cache: Arc<RwLock<UniqueCache<Self>>>
   ) -> (*const RwLock<UniqueCache<Self>>, *const VTable) {
      use std::mem;
      use std::ptr;

      let trait_obj: *const dyn DynTwoWayUniqueCache = Arc::into_raw(unique_cache);
      let unique_cache_address = trait_obj as *const RwLock<UniqueCache<Self>>;

      let trait_object_metadata = ptr::metadata(trait_obj);
      let vtable_address = unsafe {
         mem::transmute::<_, *const ()>(trait_object_metadata)
      };

      (unique_cache_address, vtable_address)
   }
}

pub trait CloneIntoJni {
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local>;
}

pub trait CloneFromJni
   where Self: Sized
{
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> Self;
}

/// RepositoryCache<T>
impl<T> CloneIntoJni for Cache<T>
   where T: CacheContent + CloneIntoJni
{
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      self.create_jvm_instance(env)
   }
}

impl<T> CloneFromJni for Cache<T>
   where T: CacheContent
{
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> Cache<T> {
      unsafe {
         Cache::<T>::from_jvm_instance(env, &java_object)
      }
   }
}

/// T
impl<T> CloneIntoJni for Box<T>
   where T: CloneIntoJni
{
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      self.as_ref().clone_into_jni(env)
   }
}

impl<T> CloneFromJni for Box<T>
   where T: CloneFromJni
{
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> Box<T> {
      Box::new(T::clone_from_jni(env, java_object))
   }
}

/// java.util.ArrayList (=kotlin.collections.ArrayList)
impl<T> CloneIntoJni for Vec<T>
   where T: CloneIntoJni
{
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      use jni::signature::{Primitive, ReturnType};

      let list = env.new_object("Ljava/util/ArrayList;", "()V", &[]).unwrap();

      let add_method_id = env
         .get_method_id("Ljava/util/ArrayList;", "add", "(Ljava/lang/Object;)Z")
         .unwrap();

      for element in self {
         let element = element.clone_into_jni(env);
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

impl<T> CloneFromJni for Vec<T>
   where T: CloneFromJni
{
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> Vec<T> {
      use jni::signature::ReturnType;

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

         vec.push(T::clone_from_jni(env, &element));
      }

      vec
   }
}

/// T?
impl<T> CloneIntoJni for Option<T>
   where T: CloneIntoJni
{
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      match self {
         Some(t) => t.clone_into_jni(env),
         None => JObject::null()
      }
   }
}

impl<T> CloneFromJni for Option<T>
   where T: CloneFromJni
{
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> Option<T> {
      if java_object.is_null() {
         None
      } else {
         Some(T::clone_from_jni(env, java_object))
      }
   }
}

/// java.lang.Boolean
impl CloneIntoJni for bool {
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Boolean", "valueOf", "(Z)Ljava/lang/Boolean;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }
}

impl CloneFromJni for bool {
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> bool {
      env.call_method(&java_object, "booleanValue", "()Z", &[]).unwrap().z().unwrap()
   }
}

/// java.lang.Byte
impl CloneIntoJni for i8 {
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Byte", "valueOf", "(B)Ljava/lang/Byte;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }
}

impl CloneFromJni for i8 {
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> i8 {
      env.call_method(&java_object, "byteValue", "()B", &[]).unwrap().b().unwrap()
   }
}

/// java.lang.Short
impl CloneIntoJni for i16 {
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Short", "valueOf", "(S)Ljava/lang/Short;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }
}

impl CloneFromJni for i16 {
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> i16 {
      env.call_method(&java_object, "shortValue", "()S", &[]).unwrap().s().unwrap()
   }
}

/// java.lang.Integer
impl CloneIntoJni for i32 {
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Integer", "valueOf", "(I)Ljava/lang/Integer;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }
}

impl CloneFromJni for i32 {
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> i32 {
      env.call_method(&java_object, "intValue", "()I", &[]).unwrap().i().unwrap()
   }
}

/// java.lang.Long
impl CloneIntoJni for i64 {
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Long", "valueOf", "(J)Ljava/lang/Long;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }
}

impl CloneFromJni for i64 {
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> i64 {
      env.call_method(&java_object, "longValue", "()J", &[]).unwrap().j().unwrap()
   }
}

/// java.lang.Float
impl CloneIntoJni for f32 {
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Float", "valueOf", "(F)Ljava/lang/Float;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }
}

impl CloneFromJni for f32 {
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> f32 {
      env.call_method(&java_object, "floatValue", "()F", &[]).unwrap().f().unwrap()
   }
}

/// java.lang.Double
impl CloneIntoJni for f64 {
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.call_static_method(
         "java/lang/Double", "valueOf", "(D)Ljava/lang/Double;", &[(*self).into()]
      ).unwrap().l().unwrap()
   }
}

impl CloneFromJni for f64 {
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> f64 {
      env.call_method(&java_object, "doubleValue", "()D", &[]).unwrap().d().unwrap()
   }
}

/// java.lang.String
impl CloneIntoJni for String {
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.new_string(&self).unwrap().into()
   }
}

impl CloneFromJni for String {
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> String {
      env.get_string(java_object.into()).unwrap().into()
   }
}

/// kotlin.Unit
impl CloneIntoJni for () {
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      env.get_static_field("kotlin/Unit", "INSTANCE", "Lkotlin/Unit;")
         .unwrap().l().unwrap()
   }
}

impl CloneFromJni for () {
   fn clone_from_jni(_env: &mut JNIEnv, _java_object: &JObject) -> () {
      ()
   }
}

/// kotlin.Pair
impl<A, B> CloneIntoJni for (A, B)
   where A: CloneIntoJni, B: CloneIntoJni
{
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      let first  = self.0.clone_into_jni(env);
      let second = self.1.clone_into_jni(env);

      env.new_object(
         "kotlin/Pair", "(Ljava/lang/Object;Ljava/lang/Object;)V",
         &[(&first).into(), (&second).into()]
      ).unwrap()
   }
}

impl<A, B> CloneFromJni for (A, B)
   where A: CloneFromJni, B: CloneFromJni
{
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> (A, B) {
      let first = env
         .call_method(java_object, "getFirst", "()Ljava/lang/Object;", &[])
         .unwrap().l().unwrap();
      let second = env
         .call_method(java_object, "getSecond", "()Ljava/lang/Object;", &[])
         .unwrap().l().unwrap();

      (A::clone_from_jni(env, &first), B::clone_from_jni(env, &second))
   }
}

/// kotlin.Triple
impl<A, B, C> CloneIntoJni for (A, B, C)
   where A: CloneIntoJni, B: CloneIntoJni, C: CloneIntoJni
{
   fn clone_into_jni<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      let first  = self.0.clone_into_jni(env);
      let second = self.1.clone_into_jni(env);
      let third  = self.2.clone_into_jni(env);

      env.new_object(
         "kotlin/Triple", "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)V",
         &[(&first).into(), (&second).into(), (&third).into()]
      ).unwrap()
   }
}

impl<A, B, C> CloneFromJni for (A, B, C)
   where A: CloneFromJni, B: CloneFromJni, C: CloneFromJni
{
   fn clone_from_jni(env: &mut JNIEnv, java_object: &JObject) -> (A, B, C) {
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
         A::clone_from_jni(env, &first),
         B::clone_from_jni(env, &second),
         C::clone_from_jni(env, &third)
      )
   }
}
