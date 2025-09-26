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

use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use crate::unique_cache::UniqueCache;

#[cfg(feature = "jvm")]
use {
   jni::JNIEnv,
   jni::objects::JObject,
   crate::convert_jvm::{CloneIntoJvm, CloneIntoJvmHelper},
   crate::jvm_type::JvmType,
};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheId {
   pub(crate) repository_dir_path: PathBuf,
   pub(crate) file_path: PathBuf
}

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
   where for<'local>
      T: CloneIntoJvm<'local, T::JvmType<'local>>
         + Serialize
   {
      self.0.save(value);
   }

   #[cfg(not(feature = "jvm"))]
   pub fn save(&self, value: T)
   where
      T: Serialize
   {
      self.0.save(value);
   }

   #[cfg(feature = "jvm")]
   pub fn create_jvm_instance<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
      self.0.create_jvm_cache(env)
   }

   pub fn id(&self) -> CacheId {
      self.0.id()
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

   #[cfg(any(test, feature = "testable"))]
   pub fn unique_cache_ptr(&self) -> *const UniqueCache<T> {
      Arc::as_ptr(&self.0)
   }
}

impl<T: CacheContent> Debug for Cache<T>
where
   T: Debug
{
   fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
where
   T: CacheContent + Serialize
{
   fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
   where
      S: Serializer
   {
      use serde::ser::SerializeTuple;
      use crate::db::savable::Savable;

      let mut tuple = serializer.serialize_tuple(2)?;

      let repository_dir_path = self.0.repository_dir_path();
      tuple.serialize_element(repository_dir_path)?;

      let file_path = self.0.file_path();
      tuple.serialize_element(&file_path)?;

      tuple.end()
   }
}

#[cfg(not(feature = "jvm"))]
impl<'de, T> Deserialize<'de> for Cache<T>
where for<'content_de>
   T: CacheContent + Deserialize<'content_de>
{
   fn deserialize<D>(deserializer: D) -> Result<Cache<T>, D::Error>
   where
      D: Deserializer<'de>,
   {
      use std::{any, mem};
      use std::fs::File;
      use std::io::BufReader;
      use std::marker::PhantomData;
      use serde::de::{SeqAccess, Visitor};
      use serde_json::de::IoRead;
      use crate::db::loader::{CacheDeserializer, Loader};

      // TODO: TypeId::ofがT: Sizedを要求しなくなり次第そちらへ移行する
      let deserializer: &mut CacheDeserializer
         = if any::type_name::<D>() == any::type_name::<&mut CacheDeserializer>() {
            unsafe { mem::transmute_copy(&deserializer) }
         } else if any::type_name::<D>()
            == any::type_name::<&mut serde_json::Deserializer<IoRead<BufReader<File>>>>()
         {
            // CacheSerializer::deserialize_structがserde_json::Deserializerに
            // 処理を委譲するため、Cacheを含む構造体のデシリアライズ時は
            // CacheSerializerではなくserde_json::Deserializerが渡される。
            // この場合deserializerのアドレスから
            // mem::offset_of!(CacheDeserializer, deserializer)を引くことで
            // CacheSerializerのアドレスを逆算する。
            unsafe {
               let json_deserializer_ptr: usize = mem::transmute_copy(&deserializer);

               let cache_deserializer_ptr
                  = json_deserializer_ptr
                  - mem::offset_of!(CacheDeserializer, deserializer);

               &mut *(cache_deserializer_ptr as *mut CacheDeserializer)
            }
         } else {
            panic!("Caches can be deserialized only with CacheDeserializer");
         };

      debug_assert!(size_of::<D>() == size_of::<&mut CacheDeserializer>());

      struct CacheVisitor<'a, T: CacheContent> {
         loader: &'a Loader,
         _cache_content_type: PhantomData<T>
      }

      impl<'de, 'vi, T> Visitor<'de> for CacheVisitor<'vi, T>
      where for<'content_de>
         T: CacheContent + Deserialize<'content_de>
      {
         type Value = Cache<T>;

         fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
            formatter.write_str("cache file path")
         }

         fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
         where
            A: SeqAccess<'de>
         {
            let repository_dir_path = seq.next_element::<PathBuf>()?
               .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
            let file_path = seq.next_element::<PathBuf>()?
               .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;

            let cache = self.loader.load(&repository_dir_path, file_path)
               .map_err(|_| serde::de::Error::custom("could not load cache file"))?;

            Ok(cache)
         }
      }

      let CacheDeserializer { deserializer, loader } = deserializer;

      let visitor = CacheVisitor::<T> {
         loader: *loader,
         _cache_content_type: PhantomData
      };

      debug_assert!(
         size_of::<D::Error>()
            == size_of::<<&mut CacheDeserializer as Deserializer>::Error>()
      );

      let result = deserializer.deserialize_seq(visitor);
      let force_transmuted_result = unsafe { mem::transmute_copy(&result) };
      mem::forget(result);
      force_transmuted_result
   }
}

#[cfg(feature = "jvm")]
impl<'de, T> Deserialize<'de> for Cache<T>
   where for<'content_de,'local>
      T: CacheContent
         + Deserialize<'content_de>
         + CloneIntoJvm<'local, T::JvmType<'local>>
         + CloneIntoJvmHelper
{
   fn deserialize<D>(deserializer: D) -> Result<Cache<T>, D::Error>
   where
      D: Deserializer<'de>,
   {
      use std::{any, mem};
      use std::fs::File;
      use std::io::BufReader;
      use std::marker::PhantomData;
      use serde::de::{SeqAccess, Visitor};
      use serde_json::de::IoRead;
      use crate::db::loader::{CacheDeserializer, Loader};

      // TODO: TypeId::ofがT: Sizedを要求しなくなり次第そちらへ移行する
      let deserializer: &mut CacheDeserializer
         = if any::type_name::<D>() == any::type_name::<&mut CacheDeserializer>() {
            unsafe { mem::transmute_copy(&deserializer) }
         } else if any::type_name::<D>()
            == any::type_name::<&mut serde_json::Deserializer<IoRead<BufReader<File>>>>()
         {
            // CacheSerializer::deserialize_structがserde_json::Deserializerに
            // 処理を委譲するため、Cacheを含む構造体のデシリアライズ時は
            // CacheSerializerではなくserde_json::Deserializerが渡される。
            // この場合deserializerのアドレスから
            // mem::offset_of!(CacheDeserializer, deserializer)を引くことで
            // CacheSerializerのアドレスを逆算する。
            unsafe {
               let json_deserializer_ptr: usize = mem::transmute_copy(&deserializer);

               let cache_deserializer_ptr
                  = json_deserializer_ptr
                  - mem::offset_of!(CacheDeserializer, deserializer);

               &mut *(cache_deserializer_ptr as *mut CacheDeserializer)
            }
         } else {
            panic!("Caches can be deserialized only with CacheDeserializer");
         };

      debug_assert!(size_of::<D>() == size_of::<&mut CacheDeserializer>());

      struct CacheVisitor<'a, T: CacheContent> {
         loader: &'a Loader,
         _cache_content_type: PhantomData<T>
      }

      impl<'de, 'vi, T> Visitor<'de> for CacheVisitor<'vi, T>
      where for<'content_de, 'local>
         T: CacheContent
            + Deserialize<'content_de>
            + CloneIntoJvm<'local, T::JvmType<'local>>
            + CloneIntoJvmHelper
      {
         type Value = Cache<T>;

         fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
            formatter.write_str("cache file path")
         }

         fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
         where
            A: SeqAccess<'de>
         {
            let repository_dir_path = seq.next_element::<PathBuf>()?
               .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
            let file_path = seq.next_element::<PathBuf>()?
               .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;

            let cache = self.loader.load(&repository_dir_path, file_path)
               .map_err(|_| serde::de::Error::custom("could not load cache file"))?;

            Ok(cache)
         }
      }

      let CacheDeserializer { deserializer, loader } = deserializer;

      let visitor = CacheVisitor::<T> {
         loader: *loader,
         _cache_content_type: PhantomData
      };

      debug_assert!(
         size_of::<D::Error>()
            == size_of::<<&mut CacheDeserializer as Deserializer>::Error>()
      );

      let result = deserializer.deserialize_seq(visitor);
      let force_transmuted_result = unsafe { mem::transmute_copy(&result) };
      mem::forget(result);
      force_transmuted_result
   }
}

pub trait CacheContent: Send + Sync + 'static {
   type Key: Clone + Hash + Eq + Send + Sync + 'static;

   #[cfg(feature = "jvm")]
   type JvmKey<'local>: JvmType<'local>;

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
   use serde::{Deserialize, Serialize};
   use crate::cache::CacheContent;
   use crate::db::saver::Saver;
   use crate::db::scheduler::DbScheduler;
   use super::Cache;

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

   #[allow(non_snake_case)]
   #[test]
   fn saveGet() {
      use std::sync::{Arc, Weak};
      use crate::repository::Repository;

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
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
      use std::sync::{Arc, Weak};
      use crate::repository::Repository;

      let db_scheduler = Arc::new(DbScheduler::new(Saver::new()));

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         Arc::clone(&db_scheduler),
         /* loader = */ Weak::new(),
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
      use std::sync::{Arc, Weak};
      use crate::repository::Repository;

      let db_scheduler = Arc::new(DbScheduler::new(Saver::new()));

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         Arc::clone(&db_scheduler),
         /* loader = */ Weak::new(),
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
      use std::sync::{Arc, Weak};
      use crate::repository::Repository;

      #[derive(Serialize)]
      struct CacheContainer {
         cache: Cache<CacheContentImpl>
      }

      let mut repository = Repository::<CacheContentImpl>::new_testable(
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/CacheTest/serialize",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(CacheContentImpl(0, 42));
      let cache_container = CacheContainer {
         cache
      };
      let json = serde_json::to_string(&cache_container).unwrap();

      assert_eq!(
         r#"{"cache":["test/CacheTest/serialize","test/CacheTest/serialize/0"]}"#,
         &json
      );
   }

   #[test]
   fn deserialize() {
      use std::fs::{self, File};
      use std::io::BufReader;
      use std::sync::Arc;
      use scopeguard::defer;
      use serde::Deserialize;
      use serde_json::Deserializer;
      use crate::db::loader::{CacheDeserializer, Loader};
      use crate::repository::Repository;

      fs::create_dir_all("test/CacheTest/deserialize").unwrap();
      fs::write("test/CacheTest/deserialize/0", "[0,42]").unwrap();
      fs::write(
         "test/CacheTest/deserialize/container",
         r#"["test/CacheTest/deserialize","test/CacheTest/deserialize/0"]"#
      ).unwrap();

      defer! {
         fs::remove_dir_all("test/CacheTest/deserialize").unwrap()
      }

      let loader = Arc::new(Loader::new());

      let repository = Arc::new(
         Repository::<CacheContentImpl>::new_testable(
            Arc::new(DbScheduler::new(Saver::new())),
            Arc::downgrade(&loader),
            "test/CacheTest/deserialize",
            /* drop_observer = */ || ()
         )
      );

      let dyn_repo = Arc::clone(&repository);
      loader.push_repository(dyn_repo);

      let mut deserializer = CacheDeserializer::new(
         Deserializer::from_reader(BufReader::new(
            File::open("test/CacheTest/deserialize/container").unwrap()
         )),
         &loader
      );

      let cache = Cache::<CacheContentImpl>::deserialize(&mut deserializer).unwrap();

      assert_eq!(
         CacheContentImpl(0, 42),
         *cache.get()
      );
   }

   #[test]
   #[should_panic]
   fn deserialize_invalid_deserializer() {
      use serde::Deserialize;
      use serde_json::Deserializer;

      let mut deserializer = Deserializer::from_str(
         r#"["test/CacheTest/deserialize_invalid_deserializer",\
         "test/CacheTest/deserialize_invalid_deserializer/0"]"#
      );

      let _ = Cache::<CacheContentImpl>::deserialize(&mut deserializer);
   }

   #[test]
   fn deserialize_recursive() {
      use std::fs;
      use std::sync::Arc;
      use scopeguard::defer;
      use serde::Deserialize;
      use crate::db::loader::Loader;
      use crate::repository::Repository;

      #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
      struct RecursiveCacheContent(i32, Option<Cache<RecursiveCacheContent>>);

      impl CacheContent for RecursiveCacheContent {
         type Key = i32;

         fn key(&self) -> &i32 {
            &self.0
         }

         fn file_path_for_key(dir_path: &Path, key: &i32) -> PathBuf {
            dir_path.join(key.to_string())
         }
      }

      fs::create_dir_all("test/CacheTest/deserialize_recursive").unwrap();
      fs::write("test/CacheTest/deserialize_recursive/0", "[0,null]").unwrap();
      fs::write(
         "test/CacheTest/deserialize_recursive/1",
         r#"[1,["test/CacheTest/deserialize_recursive","test/CacheTest/deserialize_recursive/0"]]"#
      ).unwrap();

      defer! {
         fs::remove_dir_all("test/CacheTest/deserialize_recursive").unwrap()
      }

      let loader = Arc::new(Loader::new());

      let repository = Arc::new(
         Repository::<RecursiveCacheContent>::new_testable(
            Arc::new(DbScheduler::new(Saver::new())),
            Arc::downgrade(&loader),
            "test/CacheTest/deserialize_recursive",
            /* drop_observer = */ || ()
         )
      );

      let dyn_repo = Arc::clone(&repository);
      loader.push_repository(dyn_repo);

      let cache = repository.load(&1).unwrap();

      assert_eq!(1, cache.get().0);
      assert!(cache.get().1.is_some());

      let inner_cache = cache.get().1.as_ref().unwrap().get();
      assert_eq!(0, inner_cache.0);
      assert!(inner_cache.1.is_none());
   }

   #[test]
   fn id() {
      use std::sync::{Arc, Weak};
      use crate::repository::Repository;
      use super::CacheId;

      let repository = Repository::<CacheContentImpl>::new_testable(
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/CacheTest/id",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(CacheContentImpl(0, 42));

      assert_eq!(
         CacheId {
            repository_dir_path: PathBuf::from("test/CacheTest/id"),
            file_path: PathBuf::from("test/CacheTest/id/0")
         },
         cache.id()
      );
   }
}

#[cfg(feature = "jni-test")]
mod jni_tests {
   use std::path::{Path, PathBuf};
   use std::sync::Mutex;
   use jni::JNIEnv;
   use jni::objects::JObject;
   use serde::{Deserialize, Serialize};
   use crate::cache::CacheContent;
   use crate::convert_jvm::{CloneFromJvm, CloneIntoJvm};
   use crate::db::scheduler::DbScheduler;
   use crate::jvm_type;
   use crate::jvm_types::{JvmCache, JvmInteger};
   use crate::repository::Repository;
   use super::Cache;

   jvm_type! {
      JvmCacheContentImpl,
      JvmCacheContainer,
   }

   #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
   struct CacheContentImpl(i32, i32);

   impl CacheContent for CacheContentImpl {
      type Key = i32;
      type JvmKey<'local> = JvmInteger<'local>;
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

   #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
   struct CacheContainer(i32, Cache<CacheContentImpl>);

   impl CacheContent for CacheContainer {
      type Key = i32;
      type JvmKey<'local> = JvmInteger<'local>;
      type JvmType<'local> = JvmCacheContainer<'local>;

      fn key(&self) -> &i32 {
         &self.0
      }

      fn file_path_for_key(dir_path: &Path, key: &i32) -> PathBuf {
         dir_path.join(key.to_string())
      }
   }

   impl<'local> CloneIntoJvm<'local, JvmCacheContainer<'local>> for CacheContainer {
      fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmCacheContainer<'local> {
         use crate::jvm_type::JvmType;

         let jvm_cache: JvmCache<JvmCacheContentImpl> = self.1.clone_into_jvm(env);

         let j_object = env.new_object(
            "Lcom/wcaokaze/probosqis/panoptiqon/CacheTest$CacheContainer;",
            "(ILcom/wcaokaze/probosqis/panoptiqon/Cache;)V",
            &[self.0.into(), jvm_cache.j_object().into()]
         ).unwrap();

         unsafe { JvmCacheContainer::from_j_object(j_object) }
      }
   }

   impl<'local> CloneFromJvm<'local, JvmCacheContainer<'local>> for CacheContainer {
      fn clone_from_jvm(
         env: &mut JNIEnv<'local>,
         jvm_instance: &JvmCacheContainer<'local>
      ) -> CacheContainer {
         use crate::jvm_type::JvmType;

         let key = env
            .call_method(jvm_instance.j_object(), "getKey", "()I", &[]).unwrap()
            .i().unwrap();

         let value = env
            .call_method(jvm_instance.j_object(), "getCache", "()Lcom/wcaokaze/probosqis/panoptiqon/Cache;", &[]).unwrap()
            .l().unwrap();

         let jvm_cache = unsafe { &JvmCache::<JvmCacheContentImpl>::from_j_object(value) };
         let cache = Cache::<CacheContentImpl>::clone_from_jvm(env, jvm_cache);
         CacheContainer(key, cache)
      }
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_saveGet<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::sync::{Arc, Weak};
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
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
      use std::sync::{Arc, Weak};
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let mut repo_lock = saveGet_viaJni_repository.lock().unwrap();
      *repo_lock = Some(Repository::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
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
      let mut repo_lock = saveGet_viaJni_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().load(&0);
      assert_eq!(0, cache.unwrap().get().1);
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_saveViaRepository_1saveScheduled<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      use std::path::PathBuf;
      use std::sync::{Arc, Weak};
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let db_scheduler = Arc::new(DbScheduler::new(Saver::new()));

      let repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::clone(&db_scheduler),
         /* loader = */ Weak::new(),
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
      use std::sync::{Arc, Weak};
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let db_scheduler = Arc::new(DbScheduler::new(Saver::new()));

      let repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::clone(&db_scheduler),
         /* loader = */ Weak::new(),
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
      use std::sync::{Arc, Weak};
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
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
      use std::sync::{Arc, Weak};
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
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
      use std::sync::{Arc, Weak};
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
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
      use std::sync::{Arc, Weak};
      use crate::db::saver::Saver;
      use crate::jvm_types::JvmCache;
      use crate::repository::Repository;

      let repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
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
      use std::sync::{Arc, Weak};
      use crate::db::saver::Saver;
      use crate::jvm_types::JvmCache;
      use crate::repository::Repository;

      let repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
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
      use std::sync::{Arc, Weak};
      use crate::db::saver::Saver;
      use crate::repository::Repository;

      let repository = Repository::<CacheContentImpl>::new_testable(
         &mut env,
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/CacheTest/referenceCount_save",
         /* drop_observer = */ || ()
      );
      let cache = repository.save(CacheContentImpl(0, 42));

      cache.save(CacheContentImpl(0, 0));

      // pool内、JVM、cache、SaveTask、SaveTask
      assert_eq!(5, Arc::strong_count(&cache.0));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheTest_deserialize_00024loadCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JvmCache<'local, JvmCacheContainer<'local>> {
      use std::fs;
      use std::sync::Arc;
      use scopeguard::defer;
      use crate::db::loader::Loader;
      use crate::db::saver::Saver;
      use crate::jvm_repository_creator::JvmRepositoryCreator;

      fs::create_dir_all("test/CacheTest/deserialize/CacheContentImpl").unwrap();
      fs::write("test/CacheTest/deserialize/CacheContentImpl/0", "[0,42]").unwrap();
      fs::create_dir_all("test/CacheTest/deserialize/CacheContainer").unwrap();
      fs::write(
         "test/CacheTest/deserialize/CacheContainer/0",
         r#"[0,["test/CacheTest/deserialize/CacheContentImpl","test/CacheTest/deserialize/CacheContentImpl/0"]]"#
      ).unwrap();

      defer! {
         fs::remove_dir_all("test/CacheTest/deserialize").unwrap()
      }

      let db_scheduler = Arc::new(DbScheduler::new(Saver::new()));
      let loader = Arc::new(Loader::new());

      let cache_content_repo = Arc::new(
         Repository::<CacheContentImpl>::new_testable(
            &mut env,
            Arc::clone(&db_scheduler),
            Arc::downgrade(&loader),
            "test/CacheTest/deserialize/CacheContentImpl",
            /* drop_observer = */ || ()
         )
      );

      let cache_container_repo = Arc::new(
         Repository::<CacheContainer>::new_testable(
            &mut env,
            Arc::clone(&db_scheduler),
            Arc::downgrade(&loader),
            "test/CacheTest/deserialize/CacheContainer",
            /* drop_observer = */ || ()
         )
      );

      let dyn_cache_content_repo = Arc::clone(&cache_content_repo);
      loader.push_repository(dyn_cache_content_repo);
      let dyn_cache_container_repo = Arc::clone(&cache_container_repo);
      loader.push_repository(dyn_cache_container_repo);

      let repository_creator = JvmRepositoryCreator::new(&mut env);
      repository_creator.create_jvm_wrapper(&mut env, Arc::clone(&cache_content_repo));
      let jvm_cache_container_repository = repository_creator
         .create_jvm_wrapper(&mut env, Arc::clone(&cache_container_repo));

      Repository::<CacheContainer>::of(&mut env, &jvm_cache_container_repository)
         .load(&0).unwrap()
         .clone_into_jvm(&mut env)
   }
}
