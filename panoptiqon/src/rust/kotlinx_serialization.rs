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

#[cfg(feature = "jni-test")]
mod jni_tests {
   use std::path::{Path, PathBuf};
   use std::sync::LazyLock;
   use jni::JNIEnv;
   use jni::objects::JObject;
   use serde::{Deserialize, Serialize};
   use crate::{jvm_type, Panoptiqon};
   use crate::cache::CacheContent;
   use crate::convert_jvm::{CloneFromJvm, CloneIntoJvm};
   use crate::jvm_types::{JvmCache, JvmCacheId, JvmErased, JvmLong};

   jvm_type! {
      JvmCacheContentImpl,
   }

   #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
   struct CacheContentImpl {
      pub id: i64,
      pub content: String
   }

   impl CacheContent for CacheContentImpl {
      type Key = i64;
      type JvmKey<'local> = JvmLong<'local>;
      type JvmType<'local> = JvmCacheContentImpl<'local>;

      fn key(&self) -> &i64 {
         &self.id
      }

      fn file_path_for_key(dir_path: &Path, key: &i64) -> PathBuf {
         dir_path.join(key.to_string())
      }
   }

   impl<'local> CloneIntoJvm<'local, JvmCacheContentImpl<'local>> for CacheContentImpl {
      fn clone_into_jvm(&self, env: &mut JNIEnv<'local>) -> JvmCacheContentImpl<'local> {
         use crate::jvm_type::JvmType;

         let content = self.content.clone_into_jvm(env);

         let j_object = env.new_object(
            "Lcom/wcaokaze/probosqis/panoptiqon/CacheSerializerTest$CacheContentImpl;",
            "(JLjava/lang/String;)V",
            &[self.id.into(), content.j_object().into()]
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
         use crate::jvm_types::JvmString;

         let id = env
            .call_method(jvm_instance.j_object(), "geId", "()I", &[])
            .unwrap().j().unwrap();
         let content = env
            .call_method(jvm_instance.j_object(), "getContent", "()Ljava/lang/String;", &[])
            .unwrap().l().unwrap();

         CacheContentImpl {
            id,
            content: String::clone_from_jvm(env, unsafe { &JvmString::from_j_object(content) })
         }
      }
   }

   #[allow(non_upper_case_globals)]
   static serialize_panoptiqon: LazyLock<Panoptiqon> = LazyLock::new(|| Panoptiqon::new());

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheSerializerTest_serialize_00024saveCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JvmCache<'local, JvmCacheContentImpl<'local>> {
      let repository = serialize_panoptiqon
         .new_repository(&mut env, "CacheSerializerTest/serialize");

      let cache_content = CacheContentImpl {
         id: 0,
         content: "A".to_string()
      };

      repository.save(cache_content).clone_into_jvm(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheSerializerTest_serialize_00024loadCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>,
      cache_id: JvmCacheId<'local>
   ) -> JvmCache<'local, JvmErased<'local>> {
      load_or_throw(&mut env, &serialize_panoptiqon, cache_id)
   }

   #[allow(non_upper_case_globals)]
   static serialize_writable_panoptiqon: LazyLock<Panoptiqon> = LazyLock::new(|| Panoptiqon::new());

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheSerializerTest_serialize_1writable_00024saveCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JvmCache<'local, JvmCacheContentImpl<'local>> {
      let repository = serialize_writable_panoptiqon
         .new_repository(&mut env, "CacheSerializerTest/serialize_writable");

      let cache_content = CacheContentImpl {
         id: 0,
         content: "A".to_string()
      };

      repository.save(cache_content).clone_into_jvm(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheSerializerTest_serialize_1writable_00024loadCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>,
      cache_id: JvmCacheId<'local>
   ) -> JvmCache<'local, JvmErased<'local>> {
      load_or_throw(&mut env, &serialize_writable_panoptiqon, cache_id)
   }

   #[allow(non_upper_case_globals)]
   static deserialize_panoptiqon: LazyLock<Panoptiqon> = LazyLock::new(|| Panoptiqon::new());

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheSerializerTest_deserialize_00024preparePanoptiqon<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      let repository = deserialize_panoptiqon
         .new_repository(&mut env, "CacheSerializerTest/deserialize");

      let cache_content = CacheContentImpl {
         id: 0,
         content: "A".to_string()
      };

      repository.save(cache_content);
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheSerializerTest_deserialize_00024loadCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>,
      cache_id: JvmCacheId<'local>
   ) -> JvmCache<'local, JvmErased<'local>> {
      load_or_throw(&mut env, &deserialize_panoptiqon, cache_id)
   }

   #[allow(non_upper_case_globals)]
   static deserialize_writable_panoptiqon: LazyLock<Panoptiqon> = LazyLock::new(|| Panoptiqon::new());

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheSerializerTest_deserialize_1writable_00024preparePanoptiqon<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) {
      let repository = deserialize_writable_panoptiqon
         .new_repository(&mut env, "CacheSerializerTest/deserialize_writable");

      let cache_content = CacheContentImpl {
         id: 0,
         content: "A".to_string()
      };

      repository.save(cache_content);
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_CacheSerializerTest_deserialize_1writable_00024loadCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>,
      cache_id: JvmCacheId<'local>
   ) -> JvmCache<'local, JvmErased<'local>> {
      load_or_throw(&mut env, &deserialize_writable_panoptiqon, cache_id)
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
