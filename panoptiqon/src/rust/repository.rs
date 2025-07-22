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

use std::any::TypeId;
use std::path::Path;
use std::sync::{Arc, Mutex, Weak};
use serde::{Deserialize, Serialize};
use crate::cache::{Cache, CacheContent};
use crate::db::loader::Loader;
use crate::db::saver;
use crate::db::scheduler::DbScheduler;
use crate::pool::UniqueCachePool;

#[cfg(feature = "jvm")]
use {
   jni::JNIEnv,
   jni::objects::{JClass, JMethodID, JObject},
   jni::sys::jlong,
   crate::convert_jvm::CloneIntoJvm,
   crate::convert_jvm::CloneIntoJvmHelper,
   crate::jvm_type::JvmType,
   crate::jvm_types::JvmRepository,
};

pub struct Repository<T: CacheContent> {
   pool: UniqueCachePool<T>,
   loader: Weak<Loader>,
   #[cfg(any(test, feature = "testable"))]
   drop_observer: Box<dyn FnOnce() -> () + Send + Sync>
}

impl<T: CacheContent> Repository<T> {
   fn _load_file(&self, file_path: impl AsRef<Path>) -> anyhow::Result<T>
      where for<'de> T: Deserialize<'de>
   {
      use std::fs::File;
      use std::io::BufReader;
      use anyhow::Context;
      use crate::db::loader::CacheDeserializer;

      let loader = self.loader.upgrade().context("Loader is already dropped")?;

      let file = File::open(file_path)?;
      let reader = BufReader::new(file);
      let deserializer = serde_json::Deserializer::from_reader(reader);
      let mut deserializer = CacheDeserializer::new(
         deserializer, Arc::as_ref(&loader)
      );

      let cache_content = T::deserialize(&mut deserializer)?;
      Ok(cache_content)
   }

   #[cfg(any(test, feature = "testable"))]
   pub(crate) fn loader(&self) -> &Weak<Loader> {
      &self.loader
   }
}

#[cfg(feature = "jvm")]
impl<T: CacheContent> Repository<T> {
   pub(crate) fn new(
      env: &mut JNIEnv,
      db_scheduler: Arc<DbScheduler>,
      loader: Weak<Loader>,
      dir_path: impl AsRef<Path>
   ) -> Self
      where T: CloneIntoJvmHelper
   {
      let dir_path = saver::DirPath::new(dir_path.as_ref().to_path_buf());

      Repository {
         pool: UniqueCachePool::new(env, db_scheduler, &dir_path),
         loader,
         #[cfg(any(test, feature = "testable"))]
         drop_observer: Box::new(|| ())
      }
   }

   #[cfg(any(test, feature = "testable"))]
   pub fn new_testable(
      env: &mut JNIEnv,
      db_scheduler: Arc<DbScheduler>,
      loader: Weak<Loader>,
      dir_path: impl AsRef<Path>,
      drop_observer: impl FnOnce() -> () + Send + Sync + 'static
   ) -> Self
      where T: CloneIntoJvmHelper
   {
      let dir_path = saver::DirPath::new(dir_path.as_ref().to_path_buf());

      Repository {
         pool: UniqueCachePool::new(env, db_scheduler, &dir_path),
         loader,
         drop_observer: Box::new(drop_observer)
      }
   }

   pub fn of<'local>(
      env: &mut JNIEnv<'local>,
      jvm_instance: &JvmRepository<'local, T::JvmType<'local>>
   ) -> &'local Mutex<Self> {
      let instance_repo_ptr = env.call_method(
         jvm_instance.j_object(),
         "getNativeRepositoryAddress",
         "()J",
         &[]
      ).unwrap().j().unwrap();

      unsafe { &*(instance_repo_ptr as *const _) }
   }

   pub fn save(&mut self, value: T) -> Cache<T>
      where T: for<'local> CloneIntoJvm<'local, T::JvmType<'local>>
               + CloneIntoJvmHelper
               + Serialize
   {
      let key = T::Key::clone(value.key());
      let arc = self.pool.update(key, value);
      Cache::new(arc)
   }

   pub fn load(&mut self, key: &T::Key) -> anyhow::Result<Cache<T>>
      where for<'de, 'local> T: Deserialize<'de>
            + CloneIntoJvm<'local, T::JvmType<'local>>
            + CloneIntoJvmHelper
   {
      let Some(arc) = self.pool.get(key) else {
         let file_path = T::file_path_for_key(self.pool.dir_path(), key);
         return self.load_file(file_path);
      };

      let cache = Cache::new(arc);
      Ok(cache)
   }

   /// 指定されたパスのファイルを読み込み、デシリアライズして
   /// *そのキーがまだRepository内に存在しない場合* 追加して返却する。
   /// すでにRepository内に存在した場合、存在した方のCacheを返却する。
   ///
   /// loadが即読み込み処理を行うのに対してsaveはRepositoryへの追加だけを行い
   /// 実際のファイルへの書き込みは遅延するため、RepositoryにすでにCacheが
   /// 存在する場合そちらの方が新しいものである可能性が高いためである。
   pub(crate) fn load_file(
      &mut self,
      file_path: impl AsRef<Path>
   ) -> anyhow::Result<Cache<T>>
      where for<'de, 'local> T: Deserialize<'de>
            + CloneIntoJvm<'local, T::JvmType<'local>>
            + CloneIntoJvmHelper
   {
      let cache_content = self._load_file(file_path)?;
      let key = T::Key::clone(cache_content.key());
      let unique_cache = self.pool.insert_if_vacant(key, cache_content);
      Ok(Cache::new(unique_cache))
   }
}

pub(crate) trait DynRepository: Send + Sync {
   fn can_load(
      &self,
      dir_path: &Path,
      content_type_id: TypeId
   ) -> bool;

   /// 実装の都合上&selfを受け取るが、呼び出し後参照先のメモリ領域は
   /// 解放されている可能性がある
   #[cfg(feature = "jvm")]
   unsafe fn decrement_arc(&self);
}

impl<T: CacheContent> DynRepository for Mutex<Repository<T>> {
   fn can_load(
      &self,
      dir_path: &Path,
      content_type_id: TypeId
   ) -> bool {
      let lock = self.lock().unwrap();

      lock.pool.dir_path().as_path() == dir_path
         && content_type_id == TypeId::of::<T>()
   }

   #[cfg(feature = "jvm")]
   unsafe fn decrement_arc(&self) {
      let arc = Arc::from_raw(self as *const _);
      drop(arc);
   }
}

#[cfg(feature = "jvm")]
pub struct JvmRepositoryCreator<'local> {
   repository_class: JClass<'local>,
   constructor_id: JMethodID
}

#[cfg(feature = "jvm")]
impl<'local> JvmRepositoryCreator<'local> {
   pub fn new(env: &mut JNIEnv<'local>) -> Self {
      let repository_class =
         env.find_class("com/wcaokaze/probosqis/panoptiqon/Repository").unwrap();
      let constructor_id =
         env.get_method_id(&repository_class, "<init>", "(JJ)V").unwrap();

      Self {
         repository_class,
         constructor_id
      }
   }

   pub fn create_jvm_wrapper<T>(
      &self,
      env: &mut JNIEnv<'local>,
      repo: Arc<Mutex<Repository<T>>>,
   ) -> JvmRepository<'local, T::JvmType<'local>>
      where T: CloneIntoJvmHelper
   {
      use std::{mem, ptr};
      use jni::objects::JValue;

      /*
       * JVMインスタンスのfinalizeでArcの参照カウントをデクリメントする必要が
       * あるが、一度JVMインスタンスに管理させることで参照先の
       * 型情報(Repository<T>)が失われ、アドレスから元の構造体を復元できなくなる。
       * そのため、DynRepositoryのトレイトオブジェクトを生成し、finalize時にも
       * それを復元し、動的ディスパッチでArcをデクリメントさせる。
       */

      let dyn_repository: *const dyn DynRepository = Arc::into_raw(repo);
      let repo_address = dyn_repository as *const Repository<T>;

      let trait_object_metadata = ptr::metadata(dyn_repository);
      let vtable_address: *const () = unsafe {
         mem::transmute(trait_object_metadata)
      };

      let j_object = unsafe {
         env.new_object_unchecked(
            &self.repository_class,
            self.constructor_id,
            &[
               JValue::Long(repo_address   as jlong).as_jni(),
               JValue::Long(vtable_address as jlong).as_jni(),
            ]
         ).unwrap()
      };

      unsafe { JvmRepository::from_j_object(j_object) }
   }
}

#[cfg(not(feature = "jvm"))]
impl<T: CacheContent> Repository<T> {
   pub(crate) fn new(
      db_scheduler: Arc<DbScheduler>,
      loader: Weak<Loader>,
      dir_path: impl AsRef<Path>
   ) -> Self {
      let dir_path = saver::DirPath::new(dir_path.as_ref().to_path_buf());

      Repository {
         pool: UniqueCachePool::new(db_scheduler, &dir_path),
         loader,
         #[cfg(any(test, feature = "testable"))]
         drop_observer: Box::new(|| ())
      }
   }

   #[cfg(any(test, feature = "testable"))]
   pub fn new_testable(
      db_scheduler: Arc<DbScheduler>,
      loader: Weak<Loader>,
      dir_path: impl AsRef<Path>,
      drop_observer: impl FnOnce() -> () + Send + Sync + 'static
   ) -> Self {
      let dir_path = saver::DirPath::new(dir_path.as_ref().to_path_buf());

      Repository {
         pool: UniqueCachePool::new(db_scheduler, &dir_path),
         loader,
         drop_observer: Box::new(drop_observer)
      }
   }

   pub fn save(&mut self, value: T) -> Cache<T>
      where T: Serialize
   {
      let key = T::Key::clone(value.key());
      let arc = self.pool.update(key, value);
      Cache::new(arc)
   }

   pub fn load(&mut self, key: &T::Key) -> anyhow::Result<Cache<T>>
      where for<'de> T: Deserialize<'de>
   {
      let Some(arc) = self.pool.get(key) else {
         let file_path = T::file_path_for_key(self.pool.dir_path(), key);
         return self.load_file(file_path);
      };

      let cache = Cache::new(arc);
      Ok(cache)
   }

   /// 指定されたパスのファイルを読み込み、デシリアライズして
   /// *そのキーがまだRepository内に存在しない場合* 追加して返却する。
   /// すでにRepository内に存在した場合、存在した方のCacheを返却する。
   ///
   /// loadが即読み込み処理を行うのに対してsaveはRepositoryへの追加だけを行い
   /// 実際のファイルへの書き込みは遅延するため、RepositoryにすでにCacheが
   /// 存在する場合そちらの方が新しいものである可能性が高いためである。
   pub(crate) fn load_file(
      &mut self,
      file_path: impl AsRef<Path>
   ) -> anyhow::Result<Cache<T>>
      where for<'de> T: Deserialize<'de>
   {
      let cache_content = self._load_file(file_path)?;
      let key = T::Key::clone(cache_content.key());
      let unique_cache = self.pool.insert_if_vacant(key, cache_content);
      Ok(Cache::new(unique_cache))
   }
}

#[cfg(any(test, feature = "testable"))]
impl<T: CacheContent> Drop for Repository<T> {
   fn drop(&mut self) {
      use std::mem;

      let drop_observer = mem::replace(&mut self.drop_observer, Box::new(|| ()));
      drop_observer();
   }
}

#[cfg(feature = "jvm")]
#[no_mangle]
extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_Repository_dropNativeRepository<'local>(
   _env: JNIEnv<'local>,
   _obj: JObject<'local>,
   native_repository_address: jlong,
   vtable_address: jlong
) {
   use std::{mem, ptr};

   unsafe {
      let vtable_address = vtable_address as *const ();
      let dyn_metadata = mem::transmute(vtable_address);

      let dyn_repository: *const dyn DynRepository = ptr::from_raw_parts(
         native_repository_address as *const (), dyn_metadata
      );

      (&*dyn_repository).decrement_arc();
   };
}

#[cfg(all(test, not(feature = "jvm")))]
mod test {
   use std::path::{Path, PathBuf};
   use serde::{Deserialize, Serialize};
   use crate::cache::CacheContent;
   use super::Repository;

   #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
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

   #[test]
   fn can_load() {
      use std::any::TypeId;
      use std::sync::{Arc, Mutex, Weak};
      use crate::db::saver::{DirPath, Saver};
      use crate::db::scheduler::DbScheduler;
      use super::{DynRepository, Repository};

      struct AnotherCacheContentImpl(i32, i32);

      impl CacheContent for AnotherCacheContentImpl {
         type Key = i32;

         fn key(&self) -> &i32 {
            &self.0
         }

         fn file_path_for_key(dir_path: &Path, key: &i32) -> PathBuf {
            dir_path.join(key.to_string())
         }
      }

      let repository = Mutex::new(Repository::<CacheContentImpl>::new(
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/Repository/can_load"
      ));

      let dyn_repository: &dyn DynRepository = &repository;

      assert_eq!(
         true,
         dyn_repository.can_load(
            &DirPath::new(PathBuf::from("test/Repository/can_load")),
            TypeId::of::<CacheContentImpl>()
         )
      );

      assert_eq!(
         false,
         dyn_repository.can_load(
            &DirPath::new(PathBuf::from("unmatched/dir/path")),
            TypeId::of::<CacheContentImpl>()
         )
      );

      assert_eq!(
         false,
         dyn_repository.can_load(
            &DirPath::new(PathBuf::from("test/Repository/can_load")),
            TypeId::of::<AnotherCacheContentImpl>()
         )
      );

      assert_eq!(
         false,
         dyn_repository.can_load(
            &DirPath::new(PathBuf::from("unmatched/dir/path")),
            TypeId::of::<AnotherCacheContentImpl>()
         )
      );
   }

   #[allow(non_snake_case)]
   #[test]
   fn loadFile() {
      use std::fs;
      use std::sync::Arc;
      use scopeguard::defer;
      use crate::db::loader::Loader;
      use crate::db::saver::Saver;
      use crate::db::scheduler::DbScheduler;

      fs::create_dir_all("test/Repository/loadFile").unwrap();
      fs::write("test/Repository/loadFile/0", "[0,42]").unwrap();

      defer! {
         fs::remove_dir_all("test/Repository/loadFile").unwrap()
      }

      let loader = Arc::new(Loader::new());

      let mut repository = Repository::<CacheContentImpl>::new(
         Arc::new(DbScheduler::new(Saver::new())),
         Arc::downgrade(&loader),
         "test/Repository/loadFile"
      );

      let cache = repository.load_file("test/Repository/loadFile/0").unwrap();
      assert_eq!(
         CacheContentImpl(0, 42),
         *cache.get()
      );

      let cache2 = repository.load(&0).unwrap();
      assert!(Arc::ptr_eq(&cache.get(), &cache2.get()));
   }

   #[allow(non_snake_case)]
   #[test]
   fn loadFile_viaLoad() {
      use std::fs;
      use std::sync::Arc;
      use scopeguard::defer;
      use crate::db::loader::Loader;
      use crate::db::saver::Saver;
      use crate::db::scheduler::DbScheduler;

      fs::create_dir_all("test/Repository/loadFile_viaLoad").unwrap();
      fs::write("test/Repository/loadFile_viaLoad/0", "[0,42]").unwrap();

      defer! {
         fs::remove_dir_all("test/Repository/loadFile_viaLoad").unwrap()
      }

      let loader = Arc::new(Loader::new());

      let mut repository = Repository::<CacheContentImpl>::new(
         Arc::new(DbScheduler::new(Saver::new())),
         Arc::downgrade(&loader),
         "test/Repository/loadFile_viaLoad"
      );

      let cache = repository.load(&0).unwrap();
      assert_eq!(
         CacheContentImpl(0, 42),
         *cache.get()
      );
   }

   #[allow(non_snake_case)]
   #[test]
   fn loadFile_loaderDropped() {
      use std::fs;
      use std::sync::Arc;
      use scopeguard::defer;
      use crate::db::loader::Loader;
      use crate::db::saver::Saver;
      use crate::db::scheduler::DbScheduler;

      fs::create_dir_all("test/Repository/loadFile_loaderDropped").unwrap();
      fs::write("test/Repository/loadFile_loaderDropped/0", "[0,42]").unwrap();

      defer! {
         fs::remove_dir_all("test/Repository/loadFile_loaderDropped").unwrap()
      }

      let loader = Arc::new(Loader::new());

      let mut repository = Repository::<CacheContentImpl>::new(
         Arc::new(DbScheduler::new(Saver::new())),
         Arc::downgrade(&loader),
         "test/Repository/loadFile_loaderDropped"
      );

      drop(loader);

      let result = repository.load_file("test/Repository/loadFile_loaderDropped/0");
      assert!(result.is_err());
   }

   #[allow(non_snake_case)]
   #[test]
   fn loadFile_deserializeErr() {
      use std::fs;
      use std::sync::Arc;
      use scopeguard::defer;
      use crate::db::loader::Loader;
      use crate::db::saver::Saver;
      use crate::db::scheduler::DbScheduler;

      fs::create_dir_all("test/Repository/loadFile_deserializeErr").unwrap();

      defer! {
         fs::remove_dir_all("test/Repository/loadFile_deserializeErr").unwrap()
      }

      let loader = Arc::new(Loader::new());

      let mut repository = Repository::<CacheContentImpl>::new(
         Arc::new(DbScheduler::new(Saver::new())),
         Arc::downgrade(&loader),
         "test/Repository/loadFile_deserializeErr"
      );

      fs::write("test/Repository/loadFile_deserializeErr/0", "[0,42}").unwrap();
      let result = repository.load_file("test/Repository/loadFile_deserializeErr/0");
      assert!(result.is_err());

      fs::write("test/Repository/loadFile_deserializeErr/1", r#"{"key":1,"value":42}"#).unwrap();
      let result = repository.load_file("test/Repository/loadFile_deserializeErr/1");
      assert!(result.is_err());

      fs::write("test/Repository/loadFile_deserializeErr/2", r#"[2]"#).unwrap();
      let result = repository.load_file("test/Repository/loadFile_deserializeErr/2");
      assert!(result.is_err());
   }

   #[allow(non_snake_case)]
   #[test]
   fn loadFile_fileNotFound() {
      use std::sync::Arc;
      use crate::db::loader::Loader;
      use crate::db::saver::Saver;
      use crate::db::scheduler::DbScheduler;

      let loader = Arc::new(Loader::new());

      let mut repository = Repository::<CacheContentImpl>::new(
         Arc::new(DbScheduler::new(Saver::new())),
         Arc::downgrade(&loader),
         "test/Repository/loadFile_fileNotFound"
      );

      let result = repository.load_file("test/Repository/loadFile_fileNotFound/0");
      assert!(result.is_err());
   }

   #[allow(non_snake_case)]
   #[test]
   fn loadFile_cacheAlreadyExists() {
      use std::fs;
      use std::sync::Arc;
      use scopeguard::defer;
      use crate::db::loader::Loader;
      use crate::db::saver::Saver;
      use crate::db::scheduler::DbScheduler;

      fs::create_dir_all("test/Repository/loadFile_cacheAlreadyExists").unwrap();
      fs::write("test/Repository/loadFile_cacheAlreadyExists/0", "[0,42]").unwrap();

      defer! {
         fs::remove_dir_all("test/Repository/loadFile_cacheAlreadyExists").unwrap()
      }

      let loader = Arc::new(Loader::new());

      let mut repository = Repository::<CacheContentImpl>::new(
         Arc::new(DbScheduler::new(Saver::new())),
         Arc::downgrade(&loader),
         "test/Repository/loadFile_cacheAlreadyExists"
      );

      let cache = repository.save(CacheContentImpl(0, 0));

      let cache2 = repository.load_file("test/Repository/loadFile_cacheAlreadyExists/0").unwrap();
      assert_eq!(
         CacheContentImpl(0, 0),
         *cache.get()
      );

      assert!(Arc::ptr_eq(&cache.get(), &cache2.get()));
   }
}

#[cfg(feature = "jni-test")]
mod jni_tests {
   use std::path::{Path, PathBuf};
   use std::sync::{Arc, LazyLock, Mutex, Weak};
   use jni::JNIEnv;
   use jni::objects::JObject;
   use serde::{Deserialize, Serialize};
   use crate::cache::CacheContent;
   use crate::convert_jvm::{CloneFromJvm, CloneIntoJvm};
   use crate::db::saver::Saver;
   use crate::db::scheduler::DbScheduler;
   use crate::jvm_type;
   use crate::jvm_types::JvmRepository;
   use super::Repository;

   jvm_type! {
      JvmOneWayConversionData,
      JvmTwoWayConversionData,
   }

   #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
   struct OneWayConversionData(String, i32);

   impl CacheContent for OneWayConversionData {
      type Key = String;
      type JvmType<'local> = JvmOneWayConversionData<'local>;

      fn key(&self) -> &String {
         &self.0
      }

      fn file_path_for_key(dir_path: &Path, key: &String) -> PathBuf {
         dir_path.join(key)
      }
   }

   impl<'local> CloneIntoJvm<'local, JvmOneWayConversionData<'local>> for OneWayConversionData {
      fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmOneWayConversionData<'local> {
         use crate::jvm_type::JvmType;

         let first  = self.0.clone_into_jvm(env);
         let second = self.1;

         let j_object = env.new_object(
            "Lcom/wcaokaze/probosqis/panoptiqon/NativeRepositoryTest$OneWayConversionData;",
            "(Ljava/lang/String;I)V",
            &[first.j_string().into(), second.into()]
         ).unwrap();

         unsafe { JvmOneWayConversionData::from_j_object(j_object) }
      }
   }

   #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
   struct TwoWayConversionData(String, i32);

   impl CacheContent for TwoWayConversionData {
      type Key = String;
      type JvmType<'local> = JvmTwoWayConversionData<'local>;

      fn key(&self) -> &String {
         &self.0
      }

      fn file_path_for_key(dir_path: &Path, key: &String) -> PathBuf {
         dir_path.join(key)
      }
   }

   impl<'local> CloneIntoJvm<'local, JvmTwoWayConversionData<'local>> for TwoWayConversionData {
      fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmTwoWayConversionData<'local> {
         use crate::jvm_type::JvmType;

         let first  = self.0.clone_into_jvm(env);
         let second = self.1;

         let j_object = env.new_object(
            "Lcom/wcaokaze/probosqis/panoptiqon/NativeRepositoryTest$TwoWayConversionData;",
            "(Ljava/lang/String;I)V",
            &[first.j_string().into(), second.into()]
         ).unwrap();

         unsafe { JvmTwoWayConversionData::from_j_object(j_object) }
      }
   }

   impl<'local> CloneFromJvm<'local, JvmTwoWayConversionData<'local>> for TwoWayConversionData {
      fn clone_from_jvm(
         env: &mut JNIEnv,
         java_instance: &JvmTwoWayConversionData<'local>
      ) -> TwoWayConversionData {
         use crate::jvm_type::JvmType;
         use crate::jvm_types::JvmString;

         let first = env
            .call_method(java_instance.j_object(), "getFirst", "()Ljava/lang/String;", &[])
            .unwrap().l().unwrap();
         let second = env
            .call_method(java_instance.j_object(), "getSecond", "()I", &[])
            .unwrap().i().unwrap();

         TwoWayConversionData(
            String::clone_from_jvm(env, unsafe { &JvmString::from_j_object(first) }),
            second
         )
      }
   }

   #[allow(non_upper_case_globals)]
   static switchCacheClass_dbScheduler: LazyLock<Arc<DbScheduler>>
      = LazyLock::new(|| Arc::new(DbScheduler::new(Saver::new())));

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_switchCacheClass_00024saveOneWayData<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repository = Repository::<OneWayConversionData>::new_testable(
         &mut env,
         Arc::clone(&switchCacheClass_dbScheduler),
         /* loader = */ Weak::new(),
         "test/NativeRepositoryTest/switchCacheClass_saveOneWayData",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(OneWayConversionData("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_switchCacheClass_00024saveTwoWayData<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repository = Repository::<TwoWayConversionData>::new_testable(
         &mut env,
         Arc::clone(&switchCacheClass_dbScheduler),
         /* loader = */ Weak::new(),
         "test/NativeRepositoryTest/switchCacheClass_saveTwoWayData",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(TwoWayConversionData("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_saveLoad(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::<TwoWayConversionData>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/NativeRepositoryTest/saveLoad",
         /* drop_observer = */ || ()
      );
      repository.save(TwoWayConversionData("A".to_string(), 42));

      {
         let result = repository.load(&"A".to_string());
         assert!(result.is_ok());
         let cache = result.unwrap();
         assert_eq!(TwoWayConversionData("A".to_string(), 42), *cache.get());
      }

      repository.save(TwoWayConversionData("A".to_string(), 13));

      {
         let result = repository.load(&"A".to_string());
         assert!(result.is_ok());
         let cache = result.unwrap();
         assert_eq!(TwoWayConversionData("A".to_string(), 13), *cache.get());
      }
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_load_1noSuchCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::<TwoWayConversionData>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/NativeRepositoryTest/load_noSuchCache",
         /* drop_observer = */ || ()
      );

      let result = repository.load(&"A".to_string());
      assert!(result.is_err());
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_save_1viaCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::<TwoWayConversionData>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/NativeRepositoryTest/save_viaCache",
         /* drop_observer = */ || ()
      );

      repository.save(TwoWayConversionData("A".to_string(), 42));

      {
         let cache = repository.load(&"A".to_string()).unwrap();
         assert_eq!(TwoWayConversionData("A".to_string(), 42), *cache.get());
         cache.save(TwoWayConversionData("A".to_string(), 13));
      }

      {
         let cache = repository.load(&"A".to_string()).unwrap();
         assert_eq!(TwoWayConversionData("A".to_string(), 13), *cache.get());
      }
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_save_1affectAnotherCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::<TwoWayConversionData>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/NativeRepositoryTest/save_affectAnotherCache",
         /* drop_observer = */ || ()
      );
      repository.save(TwoWayConversionData("A".to_string(), 42));

      let cache1 = repository.load(&"A".to_string()).unwrap();
      assert_eq!(TwoWayConversionData("A".to_string(), 42), *cache1.get());

      repository.save(TwoWayConversionData("A".to_string(), 13));

      let cache2 = repository.load(&"A".to_string()).unwrap();
      assert_eq!(TwoWayConversionData("A".to_string(), 13), *cache1.get());
      assert_eq!(TwoWayConversionData("A".to_string(), 13), *cache2.get());

      cache2.save(TwoWayConversionData("A".to_string(), 0));
      assert_eq!(TwoWayConversionData("A".to_string(), 0), *cache1.get());
      assert_eq!(TwoWayConversionData("A".to_string(), 0), *cache2.get());
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_oneWay_1jvmCache_00024getCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repository = Repository::<OneWayConversionData>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/NativeRepositoryTest/oneWay_jvmCache_getCache",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(OneWayConversionData("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_twoWay_1jvmCache_00024getCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repository = Repository::<TwoWayConversionData>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/NativeRepositoryTest/twoWay_jvmCache_getCache",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(TwoWayConversionData("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[allow(non_upper_case_globals)]
   static oneWay_valueChangeFromNative_repository: Mutex<Option<Repository<OneWayConversionData>>> = Mutex::new(None);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_oneWay_1jvmCache_1valueChangeFromNative_00024getCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = oneWay_valueChangeFromNative_repository.lock().unwrap();
      *repo_lock = Some(Repository::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/NativeRepositoryTest/oneWay_jvmCache_valueChangeFromNative_getCache",
         /* drop_observer = */ || ()
      ));
      let cache = repo_lock.as_mut().unwrap().save(OneWayConversionData("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_oneWay_1jvmCache_1valueChangeFromNative_00024changeValue(
      _env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = oneWay_valueChangeFromNative_repository.lock().unwrap();
      repo_lock.as_mut().unwrap().save(OneWayConversionData("A".to_string(), 13));
   }

   #[allow(non_upper_case_globals)]
   static twoWay_valueChangeFromNative_repository: Mutex<Option<Repository<TwoWayConversionData>>> = Mutex::new(None);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_twoWay_1jvmCache_1valueChangeFromNative_00024getCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = twoWay_valueChangeFromNative_repository.lock().unwrap();
      *repo_lock = Some(Repository::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/NativeRepositoryTest/twoWay_jvmCache_valueChangeFromNative_getCache",
         /* drop_observer = */ || ()
      ));
      let cache = repo_lock.as_mut().unwrap().save(TwoWayConversionData("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_twoWay_1jvmCache_1valueChangeFromNative_00024changeValue(
      _env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = twoWay_valueChangeFromNative_repository.lock().unwrap();
      repo_lock.as_mut().unwrap().save(TwoWayConversionData("A".to_string(), 13));
   }

   #[allow(non_upper_case_globals)]
   static valueChangeFromJvm_repository: Mutex<Option<Repository<TwoWayConversionData>>> = Mutex::new(None);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_jvmCache_1valueChangeFromJvm_00024getCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = valueChangeFromJvm_repository.lock().unwrap();
      *repo_lock = Some(Repository::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/NativeRepositoryTest/jvmCache_valueChangeFromJvm_getCache",
         /* drop_observer = */ || ()
      ));
      let cache = repo_lock.as_mut().unwrap().save(TwoWayConversionData("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_jvmCache_1valueChangeFromJvm_00024assertValue(
      _env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = valueChangeFromJvm_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().load(&"A".to_string());
      assert!(cache.is_ok());
      let cache = cache.unwrap();
      assert_eq!(TwoWayConversionData("A".to_string(), 13), *cache.get());
   }

   #[allow(non_upper_case_globals)]
   static oneWay_valueChange_doesntAffectOtherKeyCaches_repository: Mutex<Option<Repository<OneWayConversionData>>> = Mutex::new(None);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_oneWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024createRepository(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = oneWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      *repo_lock = Some(Repository::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/NativeRepositoryTest/jvmCache_valueChange_doesntAffectOtherKeyCaches_createRepository",
         /* drop_observer = */ || ()
      ));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_oneWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024getCacheA<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = oneWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().save(OneWayConversionData("A".to_string(), 0));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_oneWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024getCacheB<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = oneWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().save(OneWayConversionData("B".to_string(), 1));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_oneWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024getCacheC<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = oneWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().save(OneWayConversionData("C".to_string(), 2));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_oneWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024changeCacheB(
      _env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = oneWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      repo_lock.as_mut().unwrap().save(OneWayConversionData("B".to_string(), 3));
   }

   #[allow(non_upper_case_globals)]
   static twoWay_valueChange_doesntAffectOtherKeyCaches_repository: Mutex<Option<Repository<TwoWayConversionData>>> = Mutex::new(None);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_twoWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024createRepository(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = twoWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      *repo_lock = Some(Repository::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/NativeRepositoryTest/twoWay_jvmCache_valueChange_doesntAffectOtherKeyCaches_createRepository",
         /* drop_observer = */ || ()
      ));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_twoWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024getCacheA<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = twoWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().save(TwoWayConversionData("A".to_string(), 0));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_twoWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024getCacheB<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = twoWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().save(TwoWayConversionData("B".to_string(), 1));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_twoWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024getCacheC<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = twoWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().save(TwoWayConversionData("C".to_string(), 2));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_twoWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024changeCacheB(
      _env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = twoWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      repo_lock.as_mut().unwrap().save(TwoWayConversionData("B".to_string(), 3));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_twoWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024assertCacheB(
      _env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = twoWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().load(&"B".to_string());
      assert!(cache.is_ok());
      let cache = cache.unwrap();
      assert_eq!(TwoWayConversionData("B".to_string(), 4), *cache.get());
   }

   #[allow(non_upper_case_globals)]
   static restoreNativeRepositoryBorrow_repo: Mutex<usize> = Mutex::new(0);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_restoreNativeRepositoryBorrow_00024createRepository<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JvmRepository<'local, JvmTwoWayConversionData<'local>> {
      use super::JvmRepositoryCreator;

      let repository_creator = JvmRepositoryCreator::new(&mut env);
      let repo = Arc::new(Mutex::new(
         Repository::<TwoWayConversionData>::new_testable(
            &mut env,
            Arc::new(DbScheduler::new(Saver::new())),
            /* loader = */ Weak::new(),
            "test/RepositoryTest/restoreNativeRepositoryBorrow",
            /* drop_observer = */ || ()
         )
      ));
      let jvm_repository = repository_creator
         .create_jvm_wrapper(&mut env, Arc::clone(&repo));

      let mut lock = restoreNativeRepositoryBorrow_repo.lock().unwrap();
      *lock = Arc::into_raw(Arc::clone(&repo)) as usize;

      jvm_repository
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_restoreNativeRepositoryBorrow_00024assertPtr<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>,
      jvm_repository: JvmRepository<'local, JvmTwoWayConversionData<'local>>
   ) {
      let restored_repository = Repository::<TwoWayConversionData>::of(
         &mut env, &jvm_repository
      );

      let lock = restoreNativeRepositoryBorrow_repo.lock().unwrap();

      assert_eq!(restored_repository as *const _ as usize, *lock);
   }

   #[allow(non_upper_case_globals)]
   static gc_dropNativeRepository_repoExists: Mutex<bool> = Mutex::new(false);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_gc_1dropNativeRepository_00024createRepository<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JvmRepository<'local, JvmTwoWayConversionData<'local>> {
      use super::JvmRepositoryCreator;

      let repository_creator = JvmRepositoryCreator::new(&mut env);
      let repo = Arc::new(Mutex::new(
         Repository::<TwoWayConversionData>::new_testable(
            &mut env,
            Arc::new(DbScheduler::new(Saver::new())),
            /* loader = */ Weak::new(),
            "test/RepositoryTest/restoreNativeRepositoryBorrow",
            /* drop_observer = */ || {
               let mut lock = gc_dropNativeRepository_repoExists.lock().unwrap();
               *lock = false;
            }
         )
      ));
      let jvm_repository = repository_creator.create_jvm_wrapper(&mut env, repo);

      let mut lock = gc_dropNativeRepository_repoExists.lock().unwrap();
      *lock = true;

      jvm_repository
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_gc_1dropNativeRepository_00024assertDropped<'local>(
      _env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      let lock = gc_dropNativeRepository_repoExists.lock().unwrap();
      assert!(!*lock);
   }
}
