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

#![allow(incomplete_features)]
#![feature(
   get_mut_unchecked, mapped_lock_guards, never_type, ptr_metadata,
   reentrant_lock, specialization
)]

use std::cell::RefCell;
use std::path::Path;
use std::sync::{Arc, ReentrantLock};
use crate::cache::CacheContent;
use crate::db::loader::Loader;
use crate::db::scheduler::DbScheduler;
use crate::repository::Repository;

#[cfg(feature = "jvm")]
use {
   jni::JNIEnv,
   serde::{Deserialize, Serialize},
   crate::convert_jvm::{CloneFromJvm, CloneIntoJvm, CloneIntoJvmHelper},
   crate::jvm_types::{JvmCache, JvmCacheId, JvmErased},
};

#[cfg(any(test, feature = "testable"))]
use {
   crate::db::in_memory_db::InMemoryDb,
};

pub mod cache;
pub mod repository;

pub(crate) mod db;
pub(crate) mod dyn_repository;
mod pool;
mod unique_cache;

#[cfg(feature = "jvm")]
pub mod convert_jvm;
#[cfg(feature = "jvm")]
pub mod jvm_repository_creator;
#[cfg(feature = "jvm")]
pub mod jvm_type;
#[cfg(feature = "jvm")]
pub mod jvm_types;

pub struct Panoptiqon {
   #[cfg(any(test, feature = "testable"))]
   in_memory_db: Arc<ReentrantLock<RefCell<InMemoryDb>>>,
   db_scheduler: Arc<DbScheduler>,
   loader: Arc<Loader>
}

impl Panoptiqon {
   pub fn new() -> Self {
      use crate::db::saver::Saver;

      #[cfg(any(test, feature = "testable"))]
      {
         let in_memory_db = Arc::new(ReentrantLock::new(RefCell::new(InMemoryDb::new())));
         let saver = Saver::new(Arc::clone(&in_memory_db));
         Panoptiqon {
            in_memory_db,
            db_scheduler: Arc::new(DbScheduler::new(saver)),
            loader: Arc::new(Loader::new())
         }
      }

      #[cfg(not(any(test, feature = "testable")))]
      {
         Panoptiqon {
            db_scheduler: Arc::new(DbScheduler::new(Saver::new())),
            loader: Arc::new(Loader::new())
         }
      }
   }

   #[cfg(not(feature = "jvm"))]
   pub fn new_repository<T>(
      &self,
      dir_path: impl AsRef<Path>
   ) -> Arc<Repository<T>>
   where
      T: CacheContent
   {
      let repository = Arc::new(
         Repository::new(
            Arc::clone(&self.db_scheduler),
            Arc::downgrade(&self.loader),
            dir_path,
            #[cfg(any(test, feature = "testable"))]
            Arc::clone(&self.in_memory_db)
         )
      );

      let dyn_repository = Arc::clone(&repository);
      self.loader.push_repository(dyn_repository);

      repository
   }

   #[cfg(feature = "jvm")]
   pub fn new_repository<T>(
      &self,
      env: &mut JNIEnv,
      dir_path: impl AsRef<Path>
   ) -> Arc<Repository<T>>
   where
      T: CacheContent
         + CloneIntoJvmHelper
         + Serialize
         + for<'de> Deserialize<'de>
         + for<'a> CloneIntoJvm<'a, T::JvmType<'a>>
         + for<'a> CloneFromJvm<'a, T::JvmType<'a>>,
      T::Key: for<'a> CloneFromJvm<'a, T::JvmKey<'a>>
   {
      let repository = Arc::new(
         Repository::new(
            env,
            Arc::clone(&self.db_scheduler),
            Arc::downgrade(&self.loader),
            dir_path,
            #[cfg(any(test, feature = "testable"))]
            Arc::clone(&self.in_memory_db)
         )
      );

      let dyn_repository = Arc::clone(&repository);
      self.loader.push_repository(dyn_repository);

      repository
   }

   #[cfg(feature = "jvm")]
   pub fn load_jvm<'local>(
      &self,
      env: &mut JNIEnv<'local>,
      cache_id: &JvmCacheId<'local>
   ) -> anyhow::Result<JvmCache<'local, JvmErased<'local>>> {
      self.loader.load_jvm(env, cache_id)
   }
}

#[cfg(all(test, not(feature = "jvm")))]
mod test {
   use std::path::{Path, PathBuf};
   use serde::Serialize;
   use crate::cache::CacheContent;
   use super::Panoptiqon;

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
   fn newRepository_pushesIntoLoader() {
      use std::sync::{Arc, Weak};
      use crate::repository::Repository;

      let panoptiqon = Panoptiqon::new();
      let repository = panoptiqon.new_repository::<CacheContentImpl>(
         "test/Panoptiqon/newRepository_pushesIntoLoader"
      );

      assert_eq!(
         Arc::as_ptr(&panoptiqon.loader),
         Weak::as_ptr(repository.loader())
      );

      let repositories = panoptiqon.loader.repositories();
      assert_eq!(1, repositories.len());
      assert_eq!(
         Arc::as_ptr(&repositories[0]) as *const Repository<CacheContentImpl>,
         Arc::as_ptr(&repository)
      );
   }
}

#[cfg(feature = "jni-test")]
mod jni_tests {
   use std::path::{Path, PathBuf};
   use std::sync::LazyLock;
   use jni::JNIEnv;
   use jni::objects::JObject;
   use serde::{Deserialize, Serialize};
   use crate::jvm_types::{JvmCache, JvmCacheId, JvmErased, JvmInteger};
   use crate::{jvm_type, Panoptiqon};
   use crate::cache::CacheContent;
   use crate::convert_jvm::{CloneFromJvm, CloneIntoJvm};

   jvm_type! {
      JvmCacheContentA,
      JvmCacheContentB,
   }

   #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
   struct CacheContentA(i32, i32);

   impl CacheContent for CacheContentA {
      type Key = i32;
      type JvmKey<'local> = JvmInteger<'local>;
      type JvmType<'local> = JvmCacheContentA<'local>;

      fn key(&self) -> &i32 {
         &self.0
      }

      fn file_path_for_key(dir_path: &Path, key: &i32) -> PathBuf {
         dir_path.join(key.to_string())
      }
   }

   impl<'local> CloneIntoJvm<'local, JvmCacheContentA<'local>> for CacheContentA {
      fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmCacheContentA<'local> {
         use crate::jvm_type::JvmType;

         let j_object = env.new_object(
            "com/wcaokaze/probosqis/panoptiqon/PanoptiqonTest$CacheContentA",
            "(II)V",
            &[self.0.into(), self.1.into()]
         ).unwrap();

         unsafe { JvmCacheContentA::from_j_object(j_object) }
      }
   }

   impl<'local> CloneFromJvm<'local, JvmCacheContentA<'local>> for CacheContentA {
      fn clone_from_jvm(
         env: &mut JNIEnv<'local>,
         jvm_instance: &JvmCacheContentA<'local>
      ) -> CacheContentA {
         use crate::jvm_type::JvmType;

         let key = env
            .call_method(jvm_instance.j_object(), "getKey", "()I", &[])
            .unwrap().i().unwrap();
         let value = env
            .call_method(jvm_instance.j_object(), "getValue", "()I", &[])
            .unwrap().i().unwrap();

         CacheContentA(key, value)
      }
   }

   #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
   struct CacheContentB(i32, String);

   impl CacheContent for CacheContentB {
      type Key = i32;
      type JvmKey<'local> = JvmInteger<'local>;
      type JvmType<'local> = JvmCacheContentB<'local>;

      fn key(&self) -> &i32 {
         &self.0
      }

      fn file_path_for_key(dir_path: &Path, key: &i32) -> PathBuf {
         dir_path.join(key.to_string())
      }
   }

   impl<'local> CloneIntoJvm<'local, JvmCacheContentB<'local>> for CacheContentB {
      fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmCacheContentB<'local> {
         use crate::jvm_type::JvmType;

         let value = self.1.clone_into_jvm(env);

         let j_object = env.new_object(
            "com/wcaokaze/probosqis/panoptiqon/PanoptiqonTest$CacheContentB",
            "(ILjava/lang/String;)V",
            &[self.0.into(), value.j_object().into()]
         ).unwrap();

         unsafe { JvmCacheContentB::from_j_object(j_object) }
      }
   }

   impl<'local> CloneFromJvm<'local, JvmCacheContentB<'local>> for CacheContentB {
      fn clone_from_jvm(
         env: &mut JNIEnv<'local>,
         jvm_instance: &JvmCacheContentB<'local>
      ) -> CacheContentB {
         use crate::jvm_type::JvmType;
         use crate::jvm_types::JvmString;

         let key = env
            .call_method(jvm_instance.j_object(), "getKey", "()I", &[])
            .unwrap().i().unwrap();
         let value = env
            .call_method(jvm_instance.j_object(), "getValue", "()Ljava/lang/String;", &[])
            .unwrap().l().unwrap();

         CacheContentB(
            key,
            String::clone_from_jvm(env, unsafe { &JvmString::from_j_object(value) })
         )
      }
   }

   #[allow(non_upper_case_globals)]
   static loadJvm_panoptiqon: LazyLock<Panoptiqon> = LazyLock::new(|| Panoptiqon::new());

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_PanoptiqonTest_loadJvm_00024prepareRepository<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::thread;
      use std::time::Duration;

      let cache_content_a_repo = loadJvm_panoptiqon.new_repository::<CacheContentA>(
         &mut env,
         "test/PanoptiqonTest/loadJvm_a"
      );
      let cache_content_b_repo = loadJvm_panoptiqon.new_repository::<CacheContentB>(
         &mut env,
         "test/PanoptiqonTest/loadJvm_b"
      );

      cache_content_a_repo.save(CacheContentA(0, 42));
      cache_content_b_repo.save(CacheContentB(0, "Lorem ipsum".to_string()));

      // Wait for flashing
      thread::sleep(Duration::from_millis(10));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_PanoptiqonTest_loadJvm_00024load<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>,
      id: JvmCacheId<'local>
   ) -> JvmCache<'local, JvmErased<'local>> {
      load_or_throw(&mut env, &loadJvm_panoptiqon, id)
   }

   #[allow(non_upper_case_globals)]
   static loadJvm_unmatchedType_panoptiqon: LazyLock<Panoptiqon> = LazyLock::new(|| Panoptiqon::new());

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_PanoptiqonTest_loadJvm_1unmatchedType_00024prepareRepository<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::thread;
      use std::time::Duration;

      let cache_content_a_repo = loadJvm_unmatchedType_panoptiqon.new_repository::<CacheContentA>(
         &mut env,
         "test/PanoptiqonTest/loadJvm_unmatchedType_a"
      );

      cache_content_a_repo.save(CacheContentA(0, 42));

      // Wait for flashing
      thread::sleep(Duration::from_millis(10));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_PanoptiqonTest_loadJvm_1unmatchedType_00024load<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>,
      id: JvmCacheId<'local>
   ) -> JvmCache<'local, JvmErased<'local>> {
      load_or_throw(&mut env, &loadJvm_unmatchedType_panoptiqon, id)
   }

   #[allow(non_upper_case_globals)]
   static loadJvm_fileNotFound_panoptiqon: LazyLock<Panoptiqon> = LazyLock::new(|| Panoptiqon::new());

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_PanoptiqonTest_loadJvm_1fileNotFound_00024prepareRepository<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      loadJvm_fileNotFound_panoptiqon.new_repository::<CacheContentA>(
         &mut env,
         "test/PanoptiqonTest/loadJvm_fileNotFound_a"
      );
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_PanoptiqonTest_loadJvm_1fileNotFound_00024load<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>,
      id: JvmCacheId<'local>
   ) -> JvmCache<'local, JvmErased<'local>> {
      load_or_throw(&mut env, &loadJvm_fileNotFound_panoptiqon, id)
   }

   #[allow(non_upper_case_globals)]
   static loadJvm_repositoryNotFound_panoptiqon: LazyLock<Panoptiqon> = LazyLock::new(|| Panoptiqon::new());

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_PanoptiqonTest_loadJvm_1repositoryNotFound_00024prepareRepository<'local>(
      _env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_PanoptiqonTest_loadJvm_1repositoryNotFound_00024load<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>,
      id: JvmCacheId<'local>
   ) -> JvmCache<'local, JvmErased<'local>> {
      load_or_throw(&mut env, &loadJvm_repositoryNotFound_panoptiqon, id)
   }

   fn load_or_throw<'local>(
      env: &mut JNIEnv<'local>,
      panoptiqon: &Panoptiqon,
      id: JvmCacheId<'local>
   ) -> JvmCache<'local, JvmErased<'local>> {
      use jni::objects::JThrowable;
      use crate::convert_jvm::CloneIntoJvm;
      use crate::jvm_type::JvmType;

      match panoptiqon.load_jvm(env, &id) {
         Ok(cache) => cache,
         Err(e) => {
            let message = e.to_string().clone_into_jvm(env);
            let exception = JThrowable::from(
               env.new_object(
                  "java/io/IOException", "(Ljava/lang/String;)V",
                  &[message.j_string().into()]
               ).unwrap()
            );
            env.throw(exception).unwrap();
            unsafe { JvmCache::from_j_object(JObject::null()) }
         }
      }
   }
}
