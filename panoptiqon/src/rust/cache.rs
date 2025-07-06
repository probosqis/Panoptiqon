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

use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use crate::unique_cache::UniqueCache;

#[cfg(feature = "jvm")]
use {
   jni::JNIEnv,
   jni::objects::JObject,
   crate::convert_jvm::CloneIntoJvm,
   crate::jvm_type::JvmType,
};

pub struct Cache<T: CacheContent>(Arc<UniqueCache<T>>);

impl<T: CacheContent> Cache<T> {
   pub(crate) fn new(arc: Arc<UniqueCache<T>>) -> Self {
      Cache(arc)
   }

   pub fn get(&self) -> Arc<T> {
      self.0.get()
   }

   #[cfg(feature = "jvm")]
   pub fn save(&self, value: T)
      where for<'local> T: CloneIntoJvm<'local, T::JvmType<'local>>
               + Serialize
   {
      self.0.save(value);
   }

   #[cfg(not(feature = "jvm"))]
   pub fn save(&self, value: T)
      where T: Serialize
   {
      self.0.save(value);
   }

   #[cfg(feature = "jvm")]
   pub fn create_jvm_instance<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      self.0.create_jvm_cache(env)
   }

   /// RepositoryCacheのインスタンスからCacheを生成する。
   ///
   /// # Safety
   /// 指定したJVMインスタンスに対応するネイティブ側の[UniqueCache]のメモリ領域が
   /// Tと違う型を格納している場合、この関数の返り値のCacheに対するすべての動作は
   /// 未定義となる。
   #[cfg(feature = "jvm")]
   pub unsafe fn from_jvm_instance<'local>(
      env: &mut JNIEnv<'local>,
      java_instance: &JObject
   ) -> Self {
      let address = env.call_method(
         &java_instance, "getUniqueCacheRustStateAddress", "()J", &[]
      ).unwrap().j().unwrap() as *const UniqueCache<T>;

      let unique_cache = unsafe {
         Arc::increment_strong_count(address);
         Arc::<_>::from_raw(address)
      };
      Cache::new(unique_cache)
   }

   #[cfg(any(test, feature = "jni-test"))]
   pub fn unique_cache_ptr(&self) -> *const UniqueCache<T> {
      Arc::as_ptr(&self.0)
   }
}

impl<T: CacheContent> Debug for Cache<T> where T: Debug {
   fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
      let value = self.get();
      write!(f, "Cache({:?})", *value)
   }
}

impl<T: CacheContent> PartialEq for Cache<T> {
   fn eq(&self, other: &Self) -> bool {
      Arc::ptr_eq(&self.0, &other.0)
   }
}

impl<T: CacheContent> Eq for Cache<T> {}

impl<T: CacheContent> Clone for Cache<T> {
   fn clone(&self) -> Self {
      Cache(self.0.clone())
   }
}

impl<T> Serialize for Cache<T>
   where T: CacheContent + Serialize
{
   fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
      where S: Serializer
   {
      use crate::db::savable::Savable;
      let file_path = self.0.file_path();
      file_path.serialize(serializer)
   }
}

impl<'de, T: CacheContent> Deserialize<'de> for Cache<T> {
   fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
      where D: Deserializer<'de>
   {
      Err(serde::de::Error::custom("not implemented"))
   }
}

pub trait CacheContent: Send + Sync + 'static {
   type Key: Clone + Hash + Eq + Send + Sync + 'static;

   #[cfg(feature = "jvm")]
   type JvmType<'local>: JvmType<'local>;

   fn key(&self) -> &Self::Key;
   fn file_path_for_key(dir_path: &Path, key: &Self::Key) -> PathBuf;

   fn file_path(&self, dir_path: &Path) -> PathBuf {
      let key = self.key();
      Self::file_path_for_key(dir_path, key)
   }
}

#[cfg(all(test, not(feature = "jvm")))]
mod tests {
   use std::path::{Path, PathBuf};
   use serde::Serialize;
   use crate::cache::CacheContent;
   use crate::db::saver::Saver;
   use crate::db::scheduler::DbScheduler;
   use super::Cache;

   #[derive(Debug, PartialEq, Eq, Serialize)]
   struct CacheContentImpl(i32, i32);

   impl CacheContent for CacheContentImpl {
      type Key = i32;

      fn key(&self) -> &i32 {
         &self.0
      }

      fn file_path_for_key(dir_path: &Path, key: &i32) -> PathBuf {
         dir_path.join(key.to_string())
      }
   }

   #[allow(non_snake_case)]
   #[test]
   fn saveGet() {
      use std::sync::Arc;
      use crate::repository::Repository;

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         Arc::new(DbScheduler::new(Saver::new())),
         "test/CacheTest/saveGet",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(CacheContentImpl(0, 42));

      assert_eq!(42, cache.get().1);

      cache.save(CacheContentImpl(0, 0));
      assert_eq!(0, cache.get().1);

      let content = cache.get();
      cache.save(CacheContentImpl(0, 42));
      assert_eq!(42, cache.get().1);
      assert_eq!(0, content.1);
   }

   #[allow(non_snake_case)]
   #[test]
   fn saveViaRepository_saveScheduled() {
      use std::path::PathBuf;
      use std::sync::Arc;
      use crate::repository::Repository;

      let db_scheduler = Arc::new(DbScheduler::new(Saver::new()));

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         Arc::clone(&db_scheduler),
         "test/CacheTest/saveViaRepository_saveScheduled",
         /* drop_observer = */ || ()
      );

      repository.save(CacheContentImpl(0, 42));

      assert_eq!(
         vec![PathBuf::from("test/CacheTest/saveViaRepository_saveScheduled/0")],
         db_scheduler.stop().iter().map(|t| t.file_path()).collect::<Vec<_>>()
      );
   }

   #[allow(non_snake_case)]
   #[test]
   fn saveViaCache_saveScheduled() {
      use std::path::PathBuf;
      use std::sync::Arc;
      use crate::repository::Repository;

      let db_scheduler = Arc::new(DbScheduler::new(Saver::new()));

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         Arc::clone(&db_scheduler),
         "test/CacheTest/saveViaCache_saveScheduled",
         /* drop_observer = */ || ()
      );

      let cache = repository.save(CacheContentImpl(0, 42));

      cache.save(CacheContentImpl(0, 0));

      assert_eq!(
         vec![
            PathBuf::from("test/CacheTest/saveViaCache_saveScheduled/0"),
            PathBuf::from("test/CacheTest/saveViaCache_saveScheduled/0"),
         ],
         db_scheduler.stop().iter().map(|t| t.file_path()).collect::<Vec<_>>()
      );
   }

   #[test]
   fn serialize() {
      use std::sync::Arc;
      use crate::repository::Repository;

      #[derive(Serialize)]
      struct CacheContainer {
         cache: Cache<CacheContentImpl>
      }

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         Arc::new(DbScheduler::new(Saver::new())),
         "test/CacheTest/serialize",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(CacheContentImpl(0, 42));
      let cache_container = CacheContainer {
         cache
      };
      let json = serde_json::to_string(&cache_container).unwrap();

      assert_eq!(
         r#"{"cache":"test/CacheTest/serialize/0"}"#,
         &json
      );
   }
}

#[cfg(feature = "jni-test")]
mod jni_tests {
   use std::path::{Path, PathBuf};
   use std::sync::Mutex;
   use jni::JNIEnv;
   use jni::objects::JObject;
   use serde::Serialize;
   use crate::cache::CacheContent;
   use crate::convert_jvm::{CloneFromJvm, CloneIntoJvm};
   use crate::db::scheduler::DbScheduler;
   use crate::jvm_type;
   use crate::jvm_types::JvmCache;
   use crate::repository::Repository;
   use super::Cache;

   jvm_type! {
      JvmCacheContentImpl,
   }

   #[derive(Debug, PartialEq, Eq, Serialize)]
   struct CacheContentImpl(i32, i32);

   impl CacheContent for CacheContentImpl {
      type Key = i32;
      type JvmType<'local> = JvmCacheContentImpl<'local>;

      fn key(&self) -> &i32 {
         &self.0
      }

      fn file_path_for_key(dir_path: &Path, key: &i32) -> PathBuf {
         dir_path.join(key.to_string())
      }
   }

   impl<'local> CloneIntoJvm<'local, JvmCacheContentImpl<'local>> for CacheContentImpl {
      fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmCacheContentImpl<'local> {
         use crate::jvm_type::JvmType;

         let j_object = env.new_object(
            "Lcom/wcaokaze/probosqis/panoptiqon/CacheTest$CacheContentImpl;",
            "(II)V",
            &[self.0.into(), self.1.into()]
         ).unwrap();

         unsafe { JvmCacheContentImpl::from_j_object(j_object) }
      }
   }

   impl<'local> CloneFromJvm<'local, JvmCacheContentImpl<'local>> for CacheContentImpl {
      fn clone_from_jvm(
         env: &mut JNIEnv<'local>,
         jvm_instance: &JvmCacheContentImpl<'local>
      ) -> CacheContentImpl {
         use crate::jvm_type::JvmType;

         let key = env
            .call_method(jvm_instance.j_object(), "getKey", "()I", &[]).unwrap()
            .i().unwrap();

         let value = env
            .call_method(jvm_instance.j_object(), "getValue", "()I", &[]).unwrap()
            .i().unwrap();

         CacheContentImpl(key, value)
      }
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_saveGet<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::sync::Arc;
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         "test/CacheTest/saveGet",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(CacheContentImpl(0, 42));

      assert_eq!(42, cache.get().1);

      cache.save(CacheContentImpl(0, 0));
      assert_eq!(0, cache.get().1);

      let content = cache.get();
      cache.save(CacheContentImpl(0, 42));
      assert_eq!(42, cache.get().1);
      assert_eq!(0, content.1);
   }

   #[allow(non_upper_case_globals)]
   static saveGet_viaJni_repository: Mutex<Option<Repository<CacheContentImpl>>> = Mutex::new(None);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_saveGet_1viaJni_00024createRepo<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::sync::Arc;
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let mut repo_lock = saveGet_viaJni_repository.lock().unwrap();
      *repo_lock = Some(Repository::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         "test/CacheTest/saveGet_viaJni_createRepo",
         /* drop_observer = */ || ()
      ));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_saveGet_1viaJni_00024save42<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JvmCache<'local, JvmCacheContentImpl<'local>> {
      let mut repo_lock = saveGet_viaJni_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().save(CacheContentImpl(0, 42));

      cache.clone_into_jvm(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_saveGet_1viaJni_00024assert0<'local>(
      _env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      let repo_lock = saveGet_viaJni_repository.lock().unwrap();
      let cache = repo_lock.as_ref().unwrap().load(0);
      assert_eq!(0, cache.unwrap().get().1);
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_saveViaRepository_1saveScheduled<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::path::PathBuf;
      use std::sync::Arc;
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let db_scheduler = Arc::new(DbScheduler::new(Saver::new()));

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::clone(&db_scheduler),
         "test/CacheTest/saveViaRepository_saveScheduled",
         /* drop_observer = */ || ()
      );

      repository.save(CacheContentImpl(0, 42));

      assert_eq!(
         vec![PathBuf::from("test/CacheTest/saveViaRepository_saveScheduled/0")],
         db_scheduler.stop().iter().map(|t| t.file_path()).collect::<Vec<_>>()
      );
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_saveViaCache_1saveScheduled<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::path::PathBuf;
      use std::sync::Arc;
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let db_scheduler = Arc::new(DbScheduler::new(Saver::new()));

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::clone(&db_scheduler),
         "test/CacheTest/saveViaCache_saveScheduled",
         /* drop_observer = */ || ()
      );

      let cache = repository.save(CacheContentImpl(0, 42));

      cache.save(CacheContentImpl(0, 0));

      assert_eq!(
         vec![
            PathBuf::from("test/CacheTest/saveViaCache_saveScheduled/0"),
            PathBuf::from("test/CacheTest/saveViaCache_saveScheduled/0"),
         ],
         db_scheduler.stop().iter().map(|t| t.file_path()).collect::<Vec<_>>()
      );
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_referenceCount_1withoutJvmCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::sync::Arc;
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         "test/CacheTest/referenceCount_withoutJvmCache",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(CacheContentImpl(0, 42));

      // pool内、JVM、cache、SaveTask
      assert_eq!(4, Arc::strong_count(&cache.0));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_referenceCount_1withJvmCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::sync::Arc;
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         "test/CacheTest/referenceCount_withJvmCache",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(CacheContentImpl(0, 42));

      let _jvm_cache = cache.create_jvm_instance(&mut env);

      // pool内、JVM、cache、SaveTask
      assert_eq!(4, Arc::strong_count(&cache.0));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_referenceCount_1clone<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::sync::Arc;
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         "test/CacheTest/referenceCount_clone",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(CacheContentImpl(0, 42));

      let _clone = cache.clone();

      // pool内、JVM、cache、SaveTask、_clone
      assert_eq!(5, Arc::strong_count(&cache.0));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_referenceCount_1cloneIntoJvm<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::sync::Arc;
      use crate::db::saver::Saver;
      use crate::jvm_types::JvmCache;
      use crate::repository::Repository;

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         "test/CacheTest/referenceCount_cloneIntoJvm",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(CacheContentImpl(0, 42));

      let _jvm_cache: JvmCache<JvmCacheContentImpl>
         = cache.clone_into_jvm(&mut env);

      // pool内、JVM、cache、SaveTask
      assert_eq!(4, Arc::strong_count(&cache.0));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_referenceCount_1cloneIntoJvm_1cloneFromJvm<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::sync::Arc;
      use crate::db::saver::Saver;
      use crate::jvm_types::JvmCache;
      use crate::repository::Repository;

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         "test/CacheTest/referenceCount_cloneIntoJvm_cloneFromJvm",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(CacheContentImpl(0, 42));

      let jvm_cache: JvmCache<JvmCacheContentImpl>
         = cache.clone_into_jvm(&mut env);

      let _clone = Cache::<CacheContentImpl>::clone_from_jvm(&mut env, &jvm_cache);

      // pool内、JVM、cache、SaveTask、_clone
      assert_eq!(5, Arc::strong_count(&cache.0));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_referenceCount_1save<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::sync::Arc;
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         "test/CacheTest/referenceCount_save",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(CacheContentImpl(0, 42));

      cache.save(CacheContentImpl(0, 0));

      // pool内、JVM、cache、SaveTask、SaveTask
      assert_eq!(5, Arc::strong_count(&cache.0));
   }
}
