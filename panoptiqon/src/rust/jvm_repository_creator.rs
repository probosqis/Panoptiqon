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

#![cfg(feature = "jvm")]
use std::sync::Arc;
use jni::JNIEnv;
use jni::objects::{JClass, JMethodID};
use serde::Deserialize;
use crate::convert_jvm::{CloneFromJvm, CloneIntoJvm, CloneIntoJvmHelper};
use crate::jvm_types::JvmRepository;
use crate::repository::Repository;

pub struct JvmRepositoryCreator<'local> {
   repository_class: JClass<'local>,
   constructor_id: JMethodID
}

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
      repo: Arc<Repository<T>>,
   ) -> JvmRepository<'local, T::JvmType<'local>>
   where
      T: CloneIntoJvmHelper
         + for<'de> Deserialize<'de>
         + for<'a> CloneIntoJvm<'a, T::JvmType<'a>>,
      T::Key: for<'a> CloneFromJvm<'a, T::JvmKey<'a>>
   {
      use jni::objects::JValue;
      use crate::dyn_repository;
      use crate::jvm_type::JvmType;

      /*
       * JVMインスタンスのfinalizeでArcの参照カウントをデクリメントする必要が
       * あるが、一度JVMインスタンスに管理させることで参照先の
       * 型情報(Repository<T>)が失われ、アドレスから元の構造体を復元できなくなる。
       * そのため、DynRepositoryのトレイトオブジェクトを生成し、finalize時にも
       * それを復元し、動的ディスパッチでArcをデクリメントさせる。
       */

      let (repo_address, vtable_address) = dyn_repository::addresses_as_jlong(repo);

      let j_object = unsafe {
         env.new_object_unchecked(
            &self.repository_class,
            self.constructor_id,
            &[
               JValue::Long(repo_address  ).as_jni(),
               JValue::Long(vtable_address).as_jni(),
            ]
         ).unwrap()
      };

      unsafe { JvmRepository::from_j_object(j_object) }
   }
}
