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

use std::sync::Arc;
use jni::JNIEnv;
use jni::objects::JObject;
use jni::sys::jvalue;
use crate::cache::{Cache, CacheContent};
use crate::jvm_type::JvmType;
use crate::jvm_types::{
   JvmBoolean, JvmByte, JvmByteArray, JvmCache, JvmDouble, JvmFloat, JvmInteger,
   JvmList, JvmLong, JvmNullable, JvmPair, JvmShort, JvmString, JvmTriple, JvmUnit,
};
use crate::unique_cache::{
   DynOneWayUniqueCache, DynTwoWayUniqueCache, JvmUniqueCacheRefs, UniqueCache,
};

type VTable = ();

/// [CloneIntoJvm]を実装すれば自動的に実装される。
/// こちらを手で実装する必要はない
pub trait CloneIntoJvmHelper
   where Self: CacheContent + Sized
{
   #[allow(private_interfaces)]
   fn get_jvm_refs(env: &mut JNIEnv) -> JvmUniqueCacheRefs;

   fn get_unique_cache_address(
      unique_cache: Arc<UniqueCache<Self>>
   ) -> (*const UniqueCache<Self>, *const VTable);
}

impl<T> CloneIntoJvmHelper for T
   where T: CacheContent + for<'local> CloneIntoJvm<'local, T::JvmType<'local>>
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
      unique_cache: Arc<UniqueCache<Self>>
   ) -> (*const UniqueCache<Self>, *const VTable) {
      use std::mem;
      use std::ptr;

      let trait_obj: *const dyn DynOneWayUniqueCache = Arc::into_raw(unique_cache);
      let unique_cache_address = trait_obj as *const UniqueCache<Self>;

      let trait_object_metadata = ptr::metadata(trait_obj);
      let vtable_address = unsafe {
         mem::transmute::<_, *const ()>(trait_object_metadata)
      };

      (unique_cache_address, vtable_address)
   }
}

impl<T> CloneIntoJvmHelper for T
where for<'local>
      T: CacheContent
         + CloneIntoJvm<'local, T::JvmType<'local>>
         + CloneFromJvm<'local, T::JvmType<'local>>
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
      unique_cache: Arc<UniqueCache<Self>>
   ) -> (*const UniqueCache<Self>, *const VTable) {
      use std::mem;
      use std::ptr;

      let trait_obj: *const dyn DynTwoWayUniqueCache = Arc::into_raw(unique_cache);
      let unique_cache_address = trait_obj as *const UniqueCache<Self>;

      let trait_object_metadata = ptr::metadata(trait_obj);
      let vtable_address = unsafe {
         mem::transmute::<_, *const ()>(trait_object_metadata)
      };

      (unique_cache_address, vtable_address)
   }
}

pub trait CloneIntoJvm<'local, J: JvmType<'local>> {
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> J;
}

pub trait CloneFromJvm<'local, J: JvmType<'local>>
   where Self: Sized
{
   fn clone_from_jvm(env: &mut JNIEnv<'local>, jvm_instance: &J) -> Self;

   unsafe fn clone_from_j_object(
      env: &mut JNIEnv<'local>,
      j_object: &JObject<'local>
   ) -> Self
      where J: 'local
   {
      let j_object_clone = JObject::from_raw(j_object.as_raw());
      let j = J::from_j_object(j_object_clone);
      Self::clone_from_jvm(env, &j)
   }
}

/// RepositoryCache<T>
impl<'local, T, J> CloneIntoJvm<'local, JvmCache<'local, J>> for Cache<T>
   where T: CacheContent,
         J: JvmType<'local> + 'local
{
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmCache<'local, J> {
      let j_object = self.create_jvm_instance(env);
      unsafe { JvmCache::from_j_object(j_object) }
   }
}

impl<'local, T, J> CloneFromJvm<'local, JvmCache<'local, J>> for Cache<T>
   where T: CacheContent,
         J: JvmType<'local>
{
   fn clone_from_jvm(env: &mut JNIEnv, jvm_instance: &JvmCache<'local, J>) -> Cache<T> {
      unsafe {
         Cache::<T>::from_jvm_instance(env, jvm_instance.j_object())
      }
   }
}

/// T
impl<'local, T, J> CloneIntoJvm<'local, J> for &T
   where T: CloneIntoJvm<'local, J> + ?Sized,
         J: JvmType<'local>
{
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> J {
      (*self).clone_into_jvm(env)
   }
}

/// T
impl<'local, T, J> CloneIntoJvm<'local, J> for Box<T>
   where T: CloneIntoJvm<'local, J>,
         J: JvmType<'local>
{
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> J {
      self.as_ref().clone_into_jvm(env)
   }
}

impl<'local, T, J> CloneFromJvm<'local, J> for Box<T>
   where T: CloneFromJvm<'local, J>,
         J: JvmType<'local>
{
   fn clone_from_jvm(env: &mut JNIEnv<'local>, jvm_instance: &J) -> Box<T> {
      Box::new(T::clone_from_jvm(env, jvm_instance))
   }
}

/// java.util.ArrayList (=kotlin.collections.ArrayList)
impl<'local, T, J> CloneIntoJvm<'local, JvmList<'local, J>> for Vec<T>
   where T: CloneIntoJvm<'local, J>,
         J: JvmType<'local> + 'local
{
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmList<'local, J> {
      use jni::signature::{Primitive, ReturnType};

      let list = env.new_object("java/util/ArrayList", "()V", &[]).unwrap();

      let add_method_id = env
         .get_method_id("java/util/ArrayList", "add", "(Ljava/lang/Object;)Z")
         .unwrap();

      for element in self {
         let element = element.clone_into_jvm(env);
         unsafe {
            env.call_method_unchecked(
                  &list, add_method_id, ReturnType::Primitive(Primitive::Boolean),
                  &[jvalue { l: element.j_object().as_raw() }]
               )
               .unwrap();
         }
      }

      unsafe { JvmList::from_j_object(list) }
   }
}

impl<'local, T, J> CloneFromJvm<'local, JvmList<'local, J>> for Vec<T>
   where T: CloneFromJvm<'local, J>,
         J: JvmType<'local> + 'local
{
   fn clone_from_jvm(
      env: &mut JNIEnv<'local>,
      java_instance: &JvmList<'local, J>
   ) -> Vec<T> {
      use jni::signature::ReturnType;

      let mut vec = Vec::new();

      let len = env
         .call_method(java_instance.j_object(), "size", "()I", &[]).unwrap()
         .i().unwrap();

      let get_method_id = env
         .get_method_id("java/util/ArrayList", "get", "(I)Ljava/lang/Object;")
         .unwrap();

      for index in 0..len {
         let element = unsafe {
            let j_object = env.call_method_unchecked(
                  java_instance.j_object(), get_method_id, ReturnType::Object,
                  &[jvalue { i: index }]
               )
               .unwrap().l().unwrap();

            J::from_j_object(j_object)
         };

         vec.push(T::clone_from_jvm(env, &element));
      }

      vec
   }
}

/// T?
impl<'local, T, J> CloneIntoJvm<'local, JvmNullable<'local, J>> for Option<T>
   where T: CloneIntoJvm<'local, J>,
         J: JvmType<'local>
{
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmNullable<'local, J> {
      let j_object = match self {
         Some(t) => t.clone_into_jvm(env).into_j_object(),
         None => JObject::null()
      };

      unsafe { JvmNullable::from_j_object(j_object) }
   }
}

impl<'local, T, J> CloneFromJvm<'local, JvmNullable<'local, J>> for Option<T>
   where T: CloneFromJvm<'local, J>,
         J: JvmType<'local>
{
   fn clone_from_jvm(
      env: &mut JNIEnv<'local>,
      java_instance: &JvmNullable<'local, J>
   ) -> Option<T> {
      if java_instance.j_object().is_null() {
         None
      } else {
         let t = unsafe { T::clone_from_j_object(env, java_instance.j_object()) };
         Some(t)
      }
   }
}

/// java.lang.Boolean
impl<'local> CloneIntoJvm<'local, JvmBoolean<'local>> for bool {
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmBoolean<'local> {
      let j_object = env.call_static_method(
         "java/lang/Boolean", "valueOf", "(Z)Ljava/lang/Boolean;", &[(*self).into()]
      ).unwrap().l().unwrap();

      unsafe { JvmBoolean::from_j_object(j_object) }
   }
}

impl<'local> CloneFromJvm<'local, JvmBoolean<'local>> for bool {
   fn clone_from_jvm(env: &mut JNIEnv, java_instance: &JvmBoolean<'local>) -> bool {
      env.call_method(
         java_instance.j_object(), "booleanValue", "()Z", &[]
      ).unwrap().z().unwrap()
   }
}

/// java.lang.Byte
impl<'local> CloneIntoJvm<'local, JvmByte<'local>> for i8 {
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmByte<'local> {
      let j_object = env.call_static_method(
         "java/lang/Byte", "valueOf", "(B)Ljava/lang/Byte;", &[(*self).into()]
      ).unwrap().l().unwrap();

      unsafe { JvmByte::from_j_object(j_object) }
   }
}

impl<'local> CloneFromJvm<'local, JvmByte<'local>> for i8 {
   fn clone_from_jvm(env: &mut JNIEnv, java_instance: &JvmByte<'local>) -> i8 {
      env.call_method(
         java_instance.j_object(), "byteValue", "()B", &[]
      ).unwrap().b().unwrap()
   }
}

/// java.lang.Short
impl<'local> CloneIntoJvm<'local, JvmShort<'local>> for i16 {
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmShort<'local> {
      let j_object = env.call_static_method(
         "java/lang/Short", "valueOf", "(S)Ljava/lang/Short;", &[(*self).into()]
      ).unwrap().l().unwrap();

      unsafe { JvmShort::from_j_object(j_object) }
   }
}

impl<'local> CloneFromJvm<'local, JvmShort<'local>> for i16 {
   fn clone_from_jvm(env: &mut JNIEnv, java_instance: &JvmShort<'local>) -> i16 {
      env.call_method(
         java_instance.j_object(), "shortValue", "()S", &[]
      ).unwrap().s().unwrap()
   }
}

/// java.lang.Integer
impl<'local> CloneIntoJvm<'local, JvmInteger<'local>> for i32 {
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmInteger<'local> {
      let j_object = env.call_static_method(
         "java/lang/Integer", "valueOf", "(I)Ljava/lang/Integer;", &[(*self).into()]
      ).unwrap().l().unwrap();

      unsafe { JvmInteger::from_j_object(j_object) }
   }
}

impl<'local> CloneFromJvm<'local, JvmInteger<'local>> for i32 {
   fn clone_from_jvm(env: &mut JNIEnv, java_instance: &JvmInteger) -> i32 {
      env.call_method(
         java_instance.j_object(), "intValue", "()I", &[]
      ).unwrap().i().unwrap()
   }
}

/// java.lang.Long
impl<'local> CloneIntoJvm<'local, JvmLong<'local>> for i64 {
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmLong<'local> {
      let j_object = env.call_static_method(
         "java/lang/Long", "valueOf", "(J)Ljava/lang/Long;", &[(*self).into()]
      ).unwrap().l().unwrap();

      unsafe { JvmLong::from_j_object(j_object) }
   }
}

impl<'local> CloneFromJvm<'local, JvmLong<'local>> for i64 {
   fn clone_from_jvm(env: &mut JNIEnv, java_instance: &JvmLong<'local>) -> i64 {
      env.call_method(
         java_instance.j_object(), "longValue", "()J", &[]
      ).unwrap().j().unwrap()
   }
}

/// java.lang.Float
impl<'local> CloneIntoJvm<'local, JvmFloat<'local>> for f32 {
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmFloat<'local> {
      let j_object = env.call_static_method(
         "java/lang/Float", "valueOf", "(F)Ljava/lang/Float;", &[(*self).into()]
      ).unwrap().l().unwrap();

      unsafe { JvmFloat::from_j_object(j_object) }
   }
}

impl<'local> CloneFromJvm<'local, JvmFloat<'local>> for f32 {
   fn clone_from_jvm(env: &mut JNIEnv, java_instance: &JvmFloat<'local>) -> f32 {
      env.call_method(
         java_instance.j_object(), "floatValue", "()F", &[]
      ).unwrap().f().unwrap()
   }
}

/// java.lang.Double
impl<'local> CloneIntoJvm<'local, JvmDouble<'local>> for f64 {
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmDouble<'local> {
      let j_object = env.call_static_method(
         "java/lang/Double", "valueOf", "(D)Ljava/lang/Double;", &[(*self).into()]
      ).unwrap().l().unwrap();

      unsafe { JvmDouble::from_j_object(j_object) }
   }
}

impl<'local> CloneFromJvm<'local, JvmDouble<'local>> for f64 {
   fn clone_from_jvm(env: &mut JNIEnv, java_instance: &JvmDouble<'local>) -> f64 {
      env.call_method(
         java_instance.j_object(), "doubleValue", "()D", &[]
      ).unwrap().d().unwrap()
   }
}

/// byte[]
impl<'local> CloneIntoJvm<'local, JvmByteArray<'local>> for [u8] {
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmByteArray<'local> {
      let j_byte_array = env.byte_array_from_slice(&self).unwrap();
      JvmByteArray::from_j_byte_array(j_byte_array)
   }
}

impl<'local> CloneIntoJvm<'local, JvmByteArray<'local>> for Vec<u8> {
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmByteArray<'local> {
      (&self as &[u8]).clone_into_jvm(env)
   }
}

impl<'local> CloneFromJvm<'local, JvmByteArray<'local>> for Vec<u8> {
   fn clone_from_jvm(
      env: &mut JNIEnv<'local>,
      java_instance: &JvmByteArray<'local>
   ) -> Vec<u8> {
      env.convert_byte_array(java_instance.j_byte_array()).unwrap()
   }
}

/// java.lang.String
impl<'local> CloneIntoJvm<'local, JvmString<'local>> for str {
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmString<'local> {
      let j_string = env.new_string(&self).unwrap().into();
      JvmString::from_j_string(j_string)
   }
}

impl<'local> CloneIntoJvm<'local, JvmString<'local>> for String {
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmString<'local> {
      self.as_str().clone_into_jvm(env)
   }
}

impl<'local> CloneFromJvm<'local, JvmString<'local>> for String {
   fn clone_from_jvm(env: &mut JNIEnv, java_instance: &JvmString<'local>) -> String {
      env.get_string(java_instance.j_string()).unwrap().into()
   }
}

/// kotlin.Unit
impl<'local> CloneIntoJvm<'local, JvmUnit<'local>> for () {
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmUnit<'local> {
      let j_object = env.get_static_field("kotlin/Unit", "INSTANCE", "Lkotlin/Unit;")
         .unwrap().l().unwrap();

      unsafe { JvmUnit::from_j_object(j_object) }
   }
}

impl<'local> CloneFromJvm<'local, JvmUnit<'local>> for () {
   fn clone_from_jvm(_env: &mut JNIEnv, _java_instance: &JvmUnit<'local>) -> () {
      ()
   }
}

/// kotlin.Pair
impl<'local, A, B, J, K> CloneIntoJvm<'local, JvmPair<'local, J, K>> for (A, B)
   where A: CloneIntoJvm<'local, J>,
         B: CloneIntoJvm<'local, K>,
         J: JvmType<'local> + 'local,
         K: JvmType<'local> + 'local
{
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmPair<'local, J, K> {
      let first  = self.0.clone_into_jvm(env);
      let second = self.1.clone_into_jvm(env);

      let j_object = env.new_object(
         "kotlin/Pair", "(Ljava/lang/Object;Ljava/lang/Object;)V",
         &[first.j_object().into(), second.j_object().into()]
      ).unwrap();

      unsafe { JvmPair::from_j_object(j_object) }
   }
}

impl<'local, A, B, J, K> CloneFromJvm<'local, JvmPair<'local, J, K>> for (A, B)
   where A: CloneFromJvm<'local, J>,
         B: CloneFromJvm<'local, K>,
         J: JvmType<'local> + 'local,
         K: JvmType<'local> + 'local
{
   fn clone_from_jvm(
      env: &mut JNIEnv<'local>,
      java_instance: &JvmPair<'local, J, K>
   ) -> (A, B) {
      let first = env
         .call_method(java_instance.j_object(), "getFirst", "()Ljava/lang/Object;", &[])
         .unwrap().l().unwrap();
      let second = env
         .call_method(java_instance.j_object(), "getSecond", "()Ljava/lang/Object;", &[])
         .unwrap().l().unwrap();

      let j;
      let k;
      unsafe {
         j = J::from_j_object(first);
         k = K::from_j_object(second);
      }

      (
         A::clone_from_jvm(env, &j),
         B::clone_from_jvm(env, &k)
      )
   }
}

/// kotlin.Triple
impl<'local, A, B, C, J, K, L> CloneIntoJvm<'local, JvmTriple<'local, J, K, L>> for (A, B, C)
   where A: CloneIntoJvm<'local, J>,
         B: CloneIntoJvm<'local, K>,
         C: CloneIntoJvm<'local, L>,
         J: JvmType<'local> + 'local,
         K: JvmType<'local> + 'local,
         L: JvmType<'local> + 'local
{
   fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmTriple<'local, J, K, L> {
      let first  = self.0.clone_into_jvm(env);
      let second = self.1.clone_into_jvm(env);
      let third  = self.2.clone_into_jvm(env);

      let j_object = env.new_object(
         "kotlin/Triple", "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)V",
         &[first.j_object().into(), second.j_object().into(), third.j_object().into()]
      ).unwrap();

      unsafe { JvmTriple::from_j_object(j_object) }
   }
}

impl<'local, A, B, C, J, K, L> CloneFromJvm<'local, JvmTriple<'local, J, K, L>> for (A, B, C)
   where A: CloneFromJvm<'local, J>,
         B: CloneFromJvm<'local, K>,
         C: CloneFromJvm<'local, L>,
         J: JvmType<'local> + 'local,
         K: JvmType<'local> + 'local,
         L: JvmType<'local> + 'local
{
   fn clone_from_jvm(
      env: &mut JNIEnv<'local>,
      java_instance: &JvmTriple<'local, J, K, L>
   ) -> (A, B, C) {
      let first = env
         .call_method(java_instance.j_object(), "getFirst", "()Ljava/lang/Object;", &[])
         .unwrap().l().unwrap();
      let second = env
         .call_method(java_instance.j_object(), "getSecond", "()Ljava/lang/Object;", &[])
         .unwrap().l().unwrap();
      let third = env
         .call_method(java_instance.j_object(), "getThird", "()Ljava/lang/Object;", &[])
         .unwrap().l().unwrap();

      let j;
      let k;
      let l;
      unsafe {
         j = J::from_j_object(first);
         k = K::from_j_object(second);
         l = L::from_j_object(third);
      }

      (
         A::clone_from_jvm(env, &j),
         B::clone_from_jvm(env, &k),
         C::clone_from_jvm(env, &l)
      )
   }
}
