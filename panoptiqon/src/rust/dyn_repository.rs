/*
 * Copyright 2025 wcaokaze
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
use crate::cache::CacheContent;
use crate::repository::Repository;

#[cfg(feature = "jvm")]
use {
   std::sync::Arc,
   jni::JNIEnv,
   jni::sys::jlong,
   serde::Deserialize,
   crate::convert_jvm::{CloneFromJvm, CloneIntoJvm, CloneIntoJvmHelper},
   crate::jvm_types::{JvmCache, JvmErased},
};

pub(crate) trait DynRepository: Send + Sync {
   fn content_type_id(&self) -> TypeId;

   #[cfg(feature = "jvm")]
   fn load_jvm<'local>(
      &self,
      env: &mut JNIEnv<'local>,
      key: JvmErased<'local>
   ) -> anyhow::Result<JvmCache<'local, JvmErased<'local>>>;

   /// 実装の都合上&selfを受け取るが、呼び出し後参照先のメモリ領域は
   /// 解放されている可能性がある
   #[cfg(feature = "jvm")]
   unsafe fn decrement_arc(&self);
}

#[cfg(feature = "jvm")]
impl<T> DynRepository for Repository<T>
where
   T: CacheContent
      + for<'de> Deserialize<'de>
      + for<'local> CloneIntoJvm<'local, T::JvmType<'local>>
      + CloneIntoJvmHelper,
   T::Key: for<'local> CloneFromJvm<'local, T::JvmKey<'local>>
{
   fn content_type_id(&self) -> TypeId {
      TypeId::of::<T>()
   }

   fn load_jvm<'local>(
      &self,
      env: &mut JNIEnv<'local>,
      key: JvmErased<'local>
   ) -> anyhow::Result<JvmCache<'local, JvmErased<'local>>> {
      use crate::jvm_type::JvmType;

      let key = unsafe {
         let j_object = key.into_j_object();
         T::JvmKey::from_j_object(j_object)
      };
      let key = T::Key::clone_from_jvm(env, &key);
      let cache = &self.load(&key)?;

      Ok(cache.clone_into_jvm(env))
   }

   unsafe fn decrement_arc(&self) {
      let arc = Arc::from_raw(self as *const _);
      drop(arc);
   }
}

#[cfg(not(feature = "jvm"))]
impl<T> DynRepository for Repository<T>
where
   T: CacheContent
{
   fn content_type_id(&self) -> TypeId {
      TypeId::of::<T>()
   }
}

impl dyn DynRepository {
   pub(crate) fn downcast<T: CacheContent>(&self) -> Option<&Repository<T>> {
      if self.content_type_id() != TypeId::of::<T>() { return None; }

      let repository_ref = unsafe {
         let dyn_repository = self as *const dyn DynRepository;
         let repository_address = dyn_repository as *const Repository<T>;
         &*repository_address
      };

      Some(repository_ref)
   }
}

/// Arcの参照カウンタを *デクリメントせずに* 消費し、そのトレイトオブジェクトを
/// RepositoryのアドレスとVTableのアドレスに分解する。すなわち、
/// 後に[from_addresses]で復元されることを期待し、Arcを生きたまま分解する。
#[cfg(feature = "jvm")]
pub(crate) fn addresses_as_jlong<T>(
   repo: Arc<Repository<T>>
) -> (jlong, jlong)
where
   T: CacheContent
      + for<'de> Deserialize<'de>
      + for<'local> CloneIntoJvm<'local, T::JvmType<'local>>
      + CloneIntoJvmHelper,
   T::Key: for<'local> CloneFromJvm<'local, T::JvmKey<'local>>
{
   use std::{mem, ptr};
   use crate::dyn_repository::DynRepository;

   let dyn_repository: *const dyn DynRepository = Arc::into_raw(repo);
   let repo_address = dyn_repository as *const Repository<T>;

   let trait_object_metadata = ptr::metadata(dyn_repository);
   let vtable_address: *const () = unsafe {
      mem::transmute(trait_object_metadata)
   };

   (repo_address as jlong, vtable_address as jlong)
}

/// RepositoryのアドレスとVTableのアドレスから&dyn DynRepositoryを復元する。
/// [addresses_as_jlong]の逆演算
#[cfg(feature = "jvm")]
pub(crate) fn from_addresses<'local>(
   _env: &JNIEnv<'local>,
   repo_address: jlong,
   vtable_address: jlong
) -> &'local dyn DynRepository {
   use std::{mem, ptr};
   use crate::dyn_repository::DynRepository;

   unsafe {
      let vtable_address = vtable_address as *const ();
      let dyn_metadata = mem::transmute(vtable_address);

      let dyn_repository: *const dyn DynRepository = ptr::from_raw_parts(
         repo_address as *const (), dyn_metadata
      );

      &*dyn_repository
   }
}

#[cfg(all(test, not(feature = "jvm")))]
mod test {
   use std::path::{Path, PathBuf};
   use crate::cache::CacheContent;

   struct CacheContentA(i32, i32);

   impl CacheContent for CacheContentA {
      type Key = i32;

      fn key(&self) -> &i32 {
         &self.0
      }

      fn file_path_for_key(dir_path: &Path, key: &i32) -> PathBuf {
         dir_path.join(key.to_string())
      }
   }

   struct CacheContentB(i32, i32);

   impl CacheContent for CacheContentB {
      type Key = i32;

      fn key(&self) -> &i32 {
         &self.0
      }

      fn file_path_for_key(dir_path: &Path, key: &i32) -> PathBuf {
         dir_path.join(key.to_string())
      }
   }

   #[test]
   fn downcast() {
      use std::ptr;
      use std::sync::{Arc, Weak};
      use crate::db::saver::Saver;
      use crate::db::scheduler::DbScheduler;
      use crate::repository::Repository;
      use super::DynRepository;

      let repository = Repository::<CacheContentA>::new(
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/Repository/can_load"
      );

      let dyn_repository: &dyn DynRepository = &repository;

      assert!(
         ptr::eq(
            &repository,
            dyn_repository.downcast::<CacheContentA>().unwrap()
         )
      );

      assert!(
         dyn_repository.downcast::<CacheContentB>().is_none()
      );
   }
}
