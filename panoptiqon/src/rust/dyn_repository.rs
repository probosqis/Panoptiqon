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
use std::path::Path;
use crate::cache::CacheContent;
use crate::repository::Repository;

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

impl<T: CacheContent> DynRepository for Repository<T> {
   fn can_load(
      &self,
      dir_path: &Path,
      content_type_id: TypeId
   ) -> bool {
      self.pool.dir_path().as_path() == dir_path
         && content_type_id == TypeId::of::<T>()
   }

   #[cfg(feature = "jvm")]
   unsafe fn decrement_arc(&self) {
      let arc = Arc::from_raw(self as *const _);
      drop(arc);
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
   fn can_load() {
      use std::any::TypeId;
      use std::sync::{Arc, Weak};
      use crate::db::saver::{DirPath, Saver};
      use crate::db::scheduler::DbScheduler;
      use crate::repository::Repository;
      use super::DynRepository;


      let repository = Repository::<CacheContentA>::new(
         Arc::new(DbScheduler::new(Saver::new())),
         /* loader = */ Weak::new(),
         "test/Repository/can_load"
      );

      let dyn_repository: &dyn DynRepository = &repository;

      assert_eq!(
         true,
         dyn_repository.can_load(
            &DirPath::new(PathBuf::from("test/Repository/can_load")),
            TypeId::of::<CacheContentA>()
         )
      );

      assert_eq!(
         false,
         dyn_repository.can_load(
            &DirPath::new(PathBuf::from("unmatched/dir/path")),
            TypeId::of::<CacheContentA>()
         )
      );

      assert_eq!(
         false,
         dyn_repository.can_load(
            &DirPath::new(PathBuf::from("test/Repository/can_load")),
            TypeId::of::<CacheContentB>()
         )
      );

      assert_eq!(
         false,
         dyn_repository.can_load(
            &DirPath::new(PathBuf::from("unmatched/dir/path")),
            TypeId::of::<CacheContentB>()
         )
      );
   }
}
