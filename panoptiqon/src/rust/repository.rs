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
use std::hash::Hash;
use crate::cache::Cache;
use crate::pool::UniqueCachePool;

#[cfg(feature = "jvm")]
use {
   jni::JNIEnv,
   crate::convert_java::CloneIntoJava,
   crate::convert_java::CloneIntoJavaHelper,
};

pub struct Repository<K, T, S = fn(&T) -> K>
   where S: Fn(&T) -> K
{
   pool: UniqueCachePool<K, T>,
   key_selector: S
}

impl<K, T, S> Repository<K, T, S>
   where K: Hash + Eq,
         S: Fn(&T) -> K
{
   pub fn load(&mut self, key: K) -> anyhow::Result<Cache<T>> {
      let Some(arc) = self.pool.get(key) else { anyhow::bail!("not yet implemented."); };

      let cache = Cache::new(arc);
      Ok(cache)
   }
}

#[cfg(feature = "jvm")]
impl<K, T, S> Repository<K, T, S>
   where K: Hash + Eq,
         T: CloneIntoJava,
         S: Fn(&T) -> K
{
   pub fn new(
      env: &mut JNIEnv,
      key: S
   ) -> Self
      where T: CloneIntoJavaHelper
   {
      Repository {
         pool: UniqueCachePool::new(env),
         key_selector: key
      }
   }

   pub fn save(&mut self, value: T) -> Cache<T>
      where T: CloneIntoJavaHelper
   {
      let key_selector = &self.key_selector;
      let key = key_selector(&value);
      let arc = self.pool.update(key, value);
      Cache::new(arc)
   }
}

#[cfg(not(feature = "jvm"))]
impl<K, T, S> Repository<K, T, S>
   where K: Hash + Eq,
         S: Fn(&T) -> K
{
   pub fn new(key: S) -> Self {
      Repository {
         pool: UniqueCachePool::new(),
         key_selector: key
      }
   }

   pub fn save(&mut self, value: T) -> Cache<T> {
      let key_selector = &self.key_selector;
      let key = key_selector(&value);
      let arc = self.pool.update(key, value);
      Cache::new(arc)
   }
}

#[cfg(feature = "jni-test")]
mod jni_tests {
   use std::sync::Mutex;
   use jni::JNIEnv;
   use jni::objects::JObject;
   use crate::convert_java::CloneIntoJava;
   use super::Repository;

   #[derive(Debug, PartialEq, Eq)]
   struct OneWayConversionData(String, i32);

   impl CloneIntoJava for OneWayConversionData {
      fn clone_into_java<'local>(&self, env: &mut JNIEnv<'local>) -> JObject<'local> {
         let first  = self.0.clone_into_java(env);
         let second = self.1;

         env.new_object(
            "Lcom/wcaokaze/probosqis/panoptiqon/RepositoryTest$OneWayConversionData;",
            "(Ljava/lang/String;I)V",
            &[(&first).into(), second.into()]
         ).unwrap()
      }
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_switchCacheClass_00024saveOneWayData<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repository = Repository::<String, OneWayConversionData, _>::new(
         &mut env, |OneWayConversionData(k, _)| k.clone()
      );
      let cache = repository.save(OneWayConversionData("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_switchCacheClass_00024saveTwoWayData<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repository = Repository::<String, (String, i32), _>::new(
         &mut env, |(k, _)| k.clone()
      );
      let cache = repository.save(("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_saveLoad(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::<String, (String, i32), _>::new(
         &mut env, |(k, _)| k.clone()
      );
      repository.save(("A".to_string(), 42));

      {
         let result = repository.load("A".to_string());
         assert!(result.is_ok());
         let cache = result.unwrap();
         let cache_lock = cache.read().unwrap();
         assert_eq!(("A".to_string(), 42), **cache_lock);
      }

      repository.save(("A".to_string(), 13));

      {
         let result = repository.load("A".to_string());
         assert!(result.is_ok());
         let cache = result.unwrap();
         let cache_lock = cache.read().unwrap();
         assert_eq!(("A".to_string(), 13), **cache_lock);
      }
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_load_1noSuchCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::<String, (String, i32), _>::new(
         &mut env, |(k, _)| k.clone()
      );

      let result = repository.load("A".to_string());
      assert!(result.is_err());
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_save_1viaCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::<String, (String, i32), _>::new(
         &mut env, |(k, _)| k.clone()
      );
      repository.save(("A".to_string(), 42));

      {
         let cache = repository.load("A".to_string()).unwrap();
         let mut cache_lock = cache.write().unwrap();
         assert_eq!(("A".to_string(), 42), **cache_lock);
         cache_lock.save(("A".to_string(), 13));
      }

      {
         let cache = repository.load("A".to_string()).unwrap();
         let cache_lock = cache.read().unwrap();
         assert_eq!(("A".to_string(), 13), **cache_lock);
      }
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_save_1affectAnotherCache(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repository = Repository::<String, (String, i32), _>::new(
         &mut env, |(k, _)| k.clone()
      );
      repository.save(("A".to_string(), 42));

      let cache1 = repository.load("A".to_string()).unwrap();
      {
         let cache1_lock = cache1.read().unwrap();
         assert_eq!(("A".to_string(), 42), **cache1_lock);
      }

      repository.save(("A".to_string(), 13));

      let cache2 = repository.load("A".to_string()).unwrap();
      {
         let cache1_lock = cache1.read().unwrap();
         assert_eq!(("A".to_string(), 13), **cache1_lock);
      }
      {
         let mut cache2_lock = cache2.write().unwrap();
         assert_eq!(("A".to_string(), 13), **cache2_lock);
         cache2_lock.save(("A".to_string(), 0));
      }

      {
         let cache1_lock = cache1.read().unwrap();
         assert_eq!(("A".to_string(), 0), **cache1_lock);
      }
      {
         let cache2_lock = cache2.read().unwrap();
         assert_eq!(("A".to_string(), 0), **cache2_lock);
      }
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_oneWay_1jvmCache_00024getCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repository = Repository::<String, OneWayConversionData, _>::new(
         &mut env, |OneWayConversionData(k, _)| k.to_owned()
      );
      let cache = repository.save(OneWayConversionData("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_twoWay_1jvmCache_00024getCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repository = Repository::<String, (String, i32), _>::new(
         &mut env, |(k, _)| k.to_owned()
      );
      let cache = repository.save(("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[allow(non_upper_case_globals)]
   static oneWay_valueChangeFromNative_repository: Mutex<Option<Repository<String, OneWayConversionData>>> = Mutex::new(None);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_oneWay_1jvmCache_1valueChangeFromNative_00024getCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = oneWay_valueChangeFromNative_repository.lock().unwrap();
      *repo_lock = Some(Repository::new(&mut env, |OneWayConversionData(k, _)| k.to_owned()));
      let cache = repo_lock.as_mut().unwrap().save(OneWayConversionData("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_oneWay_1jvmCache_1valueChangeFromNative_00024changeValue(
      _env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = oneWay_valueChangeFromNative_repository.lock().unwrap();
      repo_lock.as_mut().unwrap().save(OneWayConversionData("A".to_string(), 13));
   }

   #[allow(non_upper_case_globals)]
   static twoWay_valueChangeFromNative_repository: Mutex<Option<Repository<String, (String, i32)>>> = Mutex::new(None);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_twoWay_1jvmCache_1valueChangeFromNative_00024getCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = twoWay_valueChangeFromNative_repository.lock().unwrap();
      *repo_lock = Some(Repository::new(&mut env, |(k, _)| k.to_owned()));
      let cache = repo_lock.as_mut().unwrap().save(("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_twoWay_1jvmCache_1valueChangeFromNative_00024changeValue(
      _env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = twoWay_valueChangeFromNative_repository.lock().unwrap();
      repo_lock.as_mut().unwrap().save(("A".to_string(), 13));
   }

   #[allow(non_upper_case_globals)]
   static valueChangeFromJvm_repository: Mutex<Option<Repository<String, (String, i32)>>> = Mutex::new(None);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_jvmCache_1valueChangeFromJvm_00024getCache<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = valueChangeFromJvm_repository.lock().unwrap();
      *repo_lock = Some(Repository::new(&mut env, |(k, _)| k.clone()));
      let cache = repo_lock.as_mut().unwrap().save(("A".to_string(), 42));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_jvmCache_1valueChangeFromJvm_00024assertValue(
      _env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = valueChangeFromJvm_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().load("A".to_string());
      assert!(cache.is_ok());
      let cache = cache.unwrap();
      let cache_lock = cache.read().unwrap();
      assert_eq!(("A".to_string(), 13), **cache_lock);
   }

   #[allow(non_upper_case_globals)]
   static oneWay_valueChange_doesntAffectOtherKeyCaches_repository: Mutex<Option<Repository<String, OneWayConversionData>>> = Mutex::new(None);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_oneWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024createRepository(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = oneWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      *repo_lock = Some(Repository::new(&mut env, |OneWayConversionData(k, _)| k.clone()));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_oneWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024getCacheA<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = oneWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().save(OneWayConversionData("A".to_string(), 0));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_oneWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024getCacheB<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = oneWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().save(OneWayConversionData("B".to_string(), 1));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_oneWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024getCacheC<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = oneWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().save(OneWayConversionData("C".to_string(), 2));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_oneWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024changeCacheB(
      _env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = oneWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      repo_lock.as_mut().unwrap().save(OneWayConversionData("B".to_string(), 3));
   }

   #[allow(non_upper_case_globals)]
   static twoWay_valueChange_doesntAffectOtherKeyCaches_repository: Mutex<Option<Repository<String, (String, i32)>>> = Mutex::new(None);

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_twoWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024createRepository(
      mut env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = twoWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      *repo_lock = Some(Repository::new(&mut env, |(k, _)| k.clone()));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_twoWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024getCacheA<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = twoWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().save(("A".to_string(), 0));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_twoWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024getCacheB<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = twoWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().save(("B".to_string(), 1));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_twoWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024getCacheC<'local>(
      mut env: JNIEnv<'local>,
      _obj: JObject<'local>
   ) -> JObject<'local> {
      let mut repo_lock = twoWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().save(("C".to_string(), 2));
      cache.create_jvm_instance(&mut env)
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_twoWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024changeCacheB(
      _env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = twoWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      repo_lock.as_mut().unwrap().save(("B".to_string(), 3));
   }

   #[no_mangle]
   extern "C" fn Java_com_wcaokaze_probosqis_panoptiqon_RepositoryTest_twoWay_1jvmCache_1valueChange_1doesntAffectOtherKeyCaches_00024assertCacheB(
      _env: JNIEnv,
      _obj: JObject
   ) {
      let mut repo_lock = twoWay_valueChange_doesntAffectOtherKeyCaches_repository.lock().unwrap();
      let cache = repo_lock.as_mut().unwrap().load("B".to_string());
      assert!(cache.is_ok());
      let cache = cache.unwrap();
      let cache_lock = cache.read().unwrap();
      assert_eq!(("B".to_string(), 4), **cache_lock);
   }
}
