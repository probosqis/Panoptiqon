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

use std::path::Path;
use serde::Serialize;
use crate::cache::{Cache, CacheContent};
use crate::db::saver;
use crate::db::scheduler::DbScheduler;
use crate::pool::UniqueCachePool;

#[cfg(feature = "jvm")]
use {
   jni::JNIEnv,
   jni::objects::JObject,
   jni::sys::jlong,
   crate::convert_jvm::CloneIntoJvm,
   crate::convert_jvm::CloneIntoJvmHelper,
   crate::jvm_type::JvmType,
   crate::jvm_types::JvmRepository,
};

pub struct Repository<T: CacheContent> {
   pool: UniqueCachePool<T>,
   #[cfg(any(test, feature = "jni-test"))]
   drop_observer: Box<dyn FnOnce() -> () + Send + Sync>
}

impl<T: CacheContent> Repository<T> {
   pub fn load(&self, key: T::Key) -> anyhow::Result<Cache<T>> {
      let Some(arc) = self.pool.get(key) else { anyhow::bail!("not yet implemented."); };

      let cache = Cache::new(arc);
      Ok(cache)
   }
}

#[cfg(feature = "jvm")]
impl<T: CacheContent> Repository<T> {
   pub fn new(env: &mut JNIEnv, dir_path: impl AsRef<Path>) -> Self
      where T: CloneIntoJvmHelper
   {
      let dir_path = saver::DirPath::new(dir_path.as_ref().to_path_buf());

      Repository {
         pool: UniqueCachePool::new(env, DbScheduler::singleton(), &dir_path),
         #[cfg(any(test, feature = "jni-test"))]
         drop_observer: Box::new(|| ())
      }
   }

   #[cfg(any(test, feature = "jni-test"))]
   pub fn new_testable(
      env: &mut JNIEnv,
      db_scheduler: &'static DbScheduler,
      dir_path: impl AsRef<Path>,
      drop_observer: impl FnOnce() -> () + Send + Sync + 'static
   ) -> Self
      where T: CloneIntoJvmHelper
   {
      let dir_path = saver::DirPath::new(dir_path.as_ref().to_path_buf());

      Repository {
         pool: UniqueCachePool::new(env, db_scheduler, &dir_path),
         drop_observer: Box::new(drop_observer)
      }
   }

   /// 新しいRepositoryを作成し、それをwrapするJVMインスタンスを生成する。
   /// Repositoryの所有権はすぐさまJVMインスタンスにムーブし、
   /// インスタンスがGCによって解放されるときにdropされる。
   fn new_jvm<'local>(
      env: &mut JNIEnv<'local>,
      dir_path: impl AsRef<Path>
   ) -> JvmRepository<'local, T::JvmType<'local>>
      where T: CloneIntoJvmHelper + 'static
   {
      let repo_box = Box::new(Repository::<T>::new(env, dir_path));
      Self::new_jvm_internal(env, repo_box).0
   }

   /// # Returns
   /// RepositoryをwrapするJVMインスタンス, wrapされたRepositoryのアドレス（テスト用）
   ///
   /// JVMインスタンスが解放されるときにネイティブ側のRepositoryもdropされるため
   /// 返り値のアドレスのライフタイムはJVMの気分次第
   #[cfg(any(test, feature = "jni-test"))]
   fn new_jvm_testable<'local>(
      env: &mut JNIEnv<'local>,
      db_scheduler: &'static DbScheduler,
      dir_path: impl AsRef<Path>,
      drop_observer: impl FnOnce() -> () + Send + Sync + 'static
   ) -> (JvmRepository<'local, T::JvmType<'local>>, *const Repository<T>)
      where T: CloneIntoJvmHelper + 'static
   {
      let repo_box = Box::new(
         Repository::<T>::new_testable(env, db_scheduler, dir_path, drop_observer)
      );
      Self::new_jvm_internal(env, repo_box)
   }

   fn new_jvm_internal<'local>(
      env: &mut JNIEnv<'local>,
      repo_box: Box<Repository<T>>
   ) -> (JvmRepository<'local, T::JvmType<'local>>, *const Repository<T>)
      where T: CloneIntoJvmHelper + 'static
   {
      use std::any::Any;
      use std::mem;

      /*
       * Repositoryを置いたヒープ領域はJVMインスタンスのfinalizeで解放する
       * 必要がある。しかし一度JVMインスタンスに管理させることで参照先の
       * 型情報(Repository<T>)が失われ、サイズがわからなくなる。
       * たいていのアロケータでは確保したメモリサイズを記録しているため
       * アプリケーション側がサイズを知っている必要はないのだが、
       * すべての環境でそうなのかについては確信がない。
       * そのため、Repositoryを置いたヒープ領域(Box<Repository<T>>)への
       * ポインタをヒープ領域に置き(Box<Box<Repository<T>>>)、それを
       * Box<dyn Any>のトレイトオブジェクトに変換することでデストラクタを
       * 実行させ、解放させる。
       */

      let mut repo_box_box = Box::new(repo_box);

      let repo_ptr: *mut _ = Box::as_mut(Box::as_mut(&mut repo_box_box));

      let trait_obj: Box<dyn Any> = repo_box_box;
      let (repo_box_ptr, vtable): (*const Box<Repository<T>>, *const ())
         = unsafe { mem::transmute(trait_obj) };

      let j_object = env.new_object(
         "com/wcaokaze/probosqis/panoptiqon/Repository",
         "(JJJ)V",
         &[
            (repo_ptr     as jlong).into(),
            (repo_box_ptr as jlong).into(),
            (vtable       as jlong).into(),
         ]
      ).unwrap();

      let jvm_repository = unsafe { JvmRepository::from_j_object(j_object) };

      (jvm_repository, repo_ptr)
   }

   fn of<'local>(
      env: &mut JNIEnv<'local>,
      jvm_instance: &JvmRepository<'local, T::JvmType<'local>>
   ) -> &'local Self {
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
               + Send + Sync
               + 'static
   {
      let key = value.key();
      let arc = self.pool.update(key, value);
      Cache::new(arc)
   }
}

#[cfg(not(feature = "jvm"))]
impl<T: CacheContent> Repository<T> {
   pub fn new(dir_path: impl AsRef<Path>) -> Self {
      let dir_path = saver::DirPath::new(dir_path.as_ref().to_path_buf());

      Repository {
         pool: UniqueCachePool::new(DbScheduler::singleton(), &dir_path),
         #[cfg(any(test, feature = "jni-test"))]
         drop_observer: Box::new(|| ())
      }
   }

   #[cfg(any(test, feature = "jni-test"))]
   pub fn new_testable(
      db_scheduler: &'static DbScheduler,
      dir_path: impl AsRef<Path>,
      drop_observer: impl FnOnce() -> () + Send + Sync + 'static
   ) -> Self {
      let dir_path = saver::DirPath::new(dir_path.as_ref().to_path_buf());

      Repository {
         pool: UniqueCachePool::new(db_scheduler, &dir_path),
         drop_observer: Box::new(drop_observer)
      }
   }

   pub fn save(&mut self, value: T) -> Cache<T>
      where T: Serialize + Send + Sync + 'static
   {
      let key = value.key();
      let arc = self.pool.update(key, value);
      Cache::new(arc)
   }
}

#[cfg(any(test, feature = "jni-test"))]
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
   native_repository_ptr_address: jlong,
   box_vtable_address: jlong
) {
   use std::mem;

   // Box<Box<Repository<T>>>のトレイトオブジェクトを復元。
   // [Repository::new_jvm_internal]参照
   let trait_obj: Box<dyn Drop> = unsafe {
      mem::transmute(
         (native_repository_ptr_address as *const (), box_vtable_address as *const ())
      )
   };
   drop(trait_obj);
}

#[cfg(feature = "jni-test")]
mod jni_tests {
   use std::sync::Mutex;
   use jni::JNIEnv;
   use jni::objects::JObject;
   use serde::Serialize;
   use crate::cache::CacheContent;
   use crate::convert_jvm::{CloneFromJvm, CloneIntoJvm};
   use crate::db::scheduler::DbScheduler;
   use crate::jvm_type;
   use crate::jvm_types::JvmRepository;
   use super::Repository;

   jvm_type! {
      JvmOneWayConversionData,
      JvmTwoWayConversionData,
   }

   #[derive(Debug, PartialEq, Eq, Serialize)]
   struct OneWayConversionData(String, i32);

   impl CacheContent for OneWayConversionData {
      type Key = String;
      type JvmType<'local> = JvmOneWayConversionData<'local>;

      fn key(&self) -> String {
         self.0.clone()
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

   #[derive(Debug, PartialEq, Eq, Serialize)]
   struct TwoWayConversionData(String, i32);

   impl CacheContent for TwoWayConversionData {
      type Key = String;
      type JvmType<'local> = JvmTwoWayConversionData<'local>;

      fn key(&self) -> String {
         self.0.clone()
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
   static switchCacheClass_dbScheduler: DbScheduler = DbScheduler::new();

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_switchCacheClass_00024saveOneWayData<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repository = Repository::<OneWayConversionData>::new_testable(
         &mut env,
         &switchCacheClass_dbScheduler,
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
         &switchCacheClass_dbScheduler,
         "test/NativeRepositoryTest/switchCacheClass_saveTwoWayData",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(TwoWayConversionData("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[allow(non_upper_case_globals)]
   static saveLoad_dbScheduler: DbScheduler = DbScheduler::new();

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_saveLoad(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::<TwoWayConversionData>::new_testable(
         &mut env,
         &saveLoad_dbScheduler,
         "test/NativeRepositoryTest/saveLoad",
         /* drop_observer = */ || ()
      );
      repository.save(TwoWayConversionData("A".to_string(), 42));

      {
         let result = repository.load("A".to_string());
         assert!(result.is_ok());
         let cache = result.unwrap();
         assert_eq!(TwoWayConversionData("A".to_string(), 42), *cache.get());
      }

      repository.save(TwoWayConversionData("A".to_string(), 13));

      {
         let result = repository.load("A".to_string());
         assert!(result.is_ok());
         let cache = result.unwrap();
         assert_eq!(TwoWayConversionData("A".to_string(), 13), *cache.get());
      }
   }

   #[allow(non_upper_case_globals)]
   static load_noSuchCache_dbScheduler: DbScheduler = DbScheduler::new();

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_load_1noSuchCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let repository = Repository::<TwoWayConversionData>::new_testable(
         &mut env,
         &load_noSuchCache_dbScheduler,
         "test/NativeRepositoryTest/load_noSuchCache",
         /* drop_observer = */ || ()
      );

      let result = repository.load("A".to_string());
      assert!(result.is_err());
   }

   #[allow(non_upper_case_globals)]
   static save_viaCache_dbScheduler: DbScheduler = DbScheduler::new();

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_save_1viaCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::<TwoWayConversionData>::new_testable(
         &mut env,
         &save_viaCache_dbScheduler,
         "test/NativeRepositoryTest/save_viaCache",
         /* drop_observer = */ || ()
      );

      repository.save(TwoWayConversionData("A".to_string(), 42));

      {
         let cache = repository.load("A".to_string()).unwrap();
         assert_eq!(TwoWayConversionData("A".to_string(), 42), *cache.get());
         cache.save(TwoWayConversionData("A".to_string(), 13));
      }

      {
         let cache = repository.load("A".to_string()).unwrap();
         assert_eq!(TwoWayConversionData("A".to_string(), 13), *cache.get());
      }
   }

   #[allow(non_upper_case_globals)]
   static save_affectAnotherCache_dbScheduler: DbScheduler = DbScheduler::new();

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_save_1affectAnotherCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::<TwoWayConversionData>::new_testable(
         &mut env,
         &save_affectAnotherCache_dbScheduler,
         "test/NativeRepositoryTest/save_affectAnotherCache",
         /* drop_observer = */ || ()
      );
      repository.save(TwoWayConversionData("A".to_string(), 42));

      let cache1 = repository.load("A".to_string()).unwrap();
      assert_eq!(TwoWayConversionData("A".to_string(), 42), *cache1.get());

      repository.save(TwoWayConversionData("A".to_string(), 13));

      let cache2 = repository.load("A".to_string()).unwrap();
      assert_eq!(TwoWayConversionData("A".to_string(), 13), *cache1.get());
      assert_eq!(TwoWayConversionData("A".to_string(), 13), *cache2.get());

      cache2.save(TwoWayConversionData("A".to_string(), 0));
      assert_eq!(TwoWayConversionData("A".to_string(), 0), *cache1.get());
      assert_eq!(TwoWayConversionData("A".to_string(), 0), *cache2.get());
   }

   #[allow(non_upper_case_globals)]
   static oneWay_jvmCache_dbScheduler: DbScheduler = DbScheduler::new();

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_oneWay_1jvmCache_00024getCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repository = Repository::<OneWayConversionData>::new_testable(
         &mut env,
         &oneWay_jvmCache_dbScheduler,
         "test/NativeRepositoryTest/oneWay_jvmCache_getCache",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(OneWayConversionData("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[allow(non_upper_case_globals)]
   static twoWay_jvmCache_dbScheduler: DbScheduler = DbScheduler::new();

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_NativeRepositoryTest_twoWay_1jvmCache_00024getCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repository = Repository::<TwoWayConversionData>::new_testable(
         &mut env,
         &twoWay_jvmCache_dbScheduler,
         "test/NativeRepositoryTest/twoWay_jvmCache_getCache",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(TwoWayConversionData("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[allow(non_upper_case_globals)]
   static oneWay_valueChangeFromNative_dbScheduler: DbScheduler = DbScheduler::new();

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
         &oneWay_valueChangeFromNative_dbScheduler,
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
   static twoWay_valueChangeFromNative_dbScheduler: DbScheduler = DbScheduler::new();

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
         &twoWay_valueChangeFromNative_dbScheduler,
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
   static valueChangeFromJvm_dbScheduler: DbScheduler = DbScheduler::new();

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
         &valueChangeFromJvm_dbScheduler,
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
      let cache = repo_lock.as_mut().unwrap().load("A".to_string());
      assert!(cache.is_ok());
      let cache = cache.unwrap();
      assert_eq!(TwoWayConversionData("A".to_string(), 13), *cache.get());
   }

   #[allow(non_upper_case_globals)]
   static oneWay_valueChange_doesntAffectOtherKeyCaches_dbScheduler: DbScheduler = DbScheduler::new();

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
         &oneWay_valueChange_doesntAffectOtherKeyCaches_dbScheduler,
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
   static twoWay_valueChange_doesntAffectOtherKeyCaches_dbScheduler: DbScheduler = DbScheduler::new();

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
         &twoWay_valueChange_doesntAffectOtherKeyCaches_dbScheduler,
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
      let cache = repo_lock.as_mut().unwrap().load("B".to_string());
      assert!(cache.is_ok());
      let cache = cache.unwrap();
      assert_eq!(TwoWayConversionData("B".to_string(), 4), *cache.get());
   }

   #[allow(non_upper_case_globals)]
   static restoreNativeRepositoryBorrow_dbScheduler: DbScheduler = DbScheduler::new();

   #[allow(non_upper_case_globals)]
   static restoreNativeRepositoryBorrow_repo: Mutex<usize> = Mutex::new(0);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_restoreNativeRepositoryBorrow_00024createRepository<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JvmRepository<'local, JvmTwoWayConversionData<'local>> {
      let (jvm_repository, repo_ptr) = Repository::<TwoWayConversionData>::new_jvm_testable(
         &mut env,
         &restoreNativeRepositoryBorrow_dbScheduler,
         "test/RepositoryTest/restoreNativeRepositoryBorrow",
         /* drop_observer = */ || ()
      );

      let mut lock = restoreNativeRepositoryBorrow_repo.lock().unwrap();
      *lock = repo_ptr as usize;

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
   static gc_dropNativeRepository_dbScheduler: DbScheduler = DbScheduler::new();

   #[allow(non_upper_case_globals)]
   static gc_dropNativeRepository_repoExists: Mutex<bool> = Mutex::new(false);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_gc_1dropNativeRepository_00024createRepository<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JvmRepository<'local, JvmTwoWayConversionData<'local>> {
      let (jvm_repository, _repo_ptr) = Repository::<TwoWayConversionData>::new_jvm_testable(
         &mut env,
         &gc_dropNativeRepository_dbScheduler,
         "test/RepositoryTest/restoreNativeRepositoryBorrow",
         /* drop_observer = */ || {
            let mut lock = gc_dropNativeRepository_repoExists.lock().unwrap();
            *lock = false;
         }
      );

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
