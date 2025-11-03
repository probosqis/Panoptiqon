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

use std::marker::PhantomData;
use jni::objects::{JByteArray, JObject, JString, JThrowable};
use crate::jvm_type;
use crate::jvm_type::JvmType;

#[repr(transparent)]
pub struct JvmString<'local>(JString<'local>);

impl<'local> JvmString<'local> {
   pub fn from_j_string(j_string: JString<'local>) -> JvmString<'local> {
      JvmString(j_string)
   }

   pub fn j_string(&self) -> &JString<'local> {
      &self.0
   }

   pub fn into_j_string(self) -> JString<'local> {
      self.0
   }
}

impl<'local> JvmType<'local> for JvmString<'local> {
   unsafe fn from_j_object(j_object: JObject<'local>) -> JvmString<'local> {
      JvmString(j_object.into())
   }

   fn j_object(&self) -> &JObject<'local> {
      &self.0
   }

   fn into_j_object(self) -> JObject<'local> {
      self.0.into()
   }
}

#[repr(transparent)]
pub struct JvmByteArray<'local>(JByteArray<'local>);

impl<'local> JvmByteArray<'local> {
   pub fn from_j_byte_array(j_byte_array: JByteArray<'local>) -> JvmByteArray<'local> {
      JvmByteArray(j_byte_array)
   }

   pub fn j_byte_array(&self) -> &JByteArray<'local> {
      &self.0
   }

   pub fn into_j_byte_array(self) -> JByteArray<'local> {
      self.0
   }
}

impl<'local> JvmType<'local> for JvmByteArray<'local> {
   unsafe fn from_j_object(j_object: JObject<'local>) -> JvmByteArray<'local> {
      JvmByteArray(j_object.into())
   }

   fn j_object(&self) -> &JObject<'local> {
      &self.0
   }

   fn into_j_object(self) -> JObject<'local> {
      self.0.into()
   }
}

#[repr(transparent)]
pub struct JvmException<'local>(JThrowable<'local>);

impl<'local> JvmException<'local> {
   pub fn from_j_throwable(j_throwable: JThrowable<'local>) -> JvmException<'local> {
      JvmException(j_throwable)
   }

   pub fn j_throwable(&self) -> &JThrowable<'local> {
      &self.0
   }

   pub fn into_j_throwable(self) -> JThrowable<'local> {
      self.0
   }
}

impl<'local> JvmType<'local> for JvmException<'local> {
   unsafe fn from_j_object(j_object: JObject<'local>) -> JvmException<'local> {
      JvmException(j_object.into())
   }

   fn j_object(&self) -> &JObject<'local> {
      &self.0
   }

   fn into_j_object(self) -> JObject<'local> {
      self.0.into()
   }
}

#[repr(transparent)]
pub struct JvmNullable<'local, T>(
   T,
   PhantomData<&'local ()>
)
where
   T: JvmType<'local> + 'local;

impl<'local, T> JvmType<'local> for JvmNullable<'local, T>
where
   T: JvmType<'local>
{
   unsafe fn from_j_object(j_object: JObject<'local>) -> JvmNullable<'local, T> {
      JvmNullable(
         T::from_j_object(j_object),
         PhantomData
      )
   }

   fn j_object(&self) -> &JObject<'local> {
      self.0.j_object()
   }

   fn into_j_object(self) -> JObject<'local> {
      self.0.into_j_object()
   }
}

#[repr(transparent)]
pub struct JvmList<'local, T: JvmType<'local>>(JObject<'local>, PhantomData<T>);

impl<'local, T: JvmType<'local>> JvmType<'local> for JvmList<'local, T> {
   unsafe fn from_j_object(j_object: JObject<'local>) -> JvmList<'local, T> {
      JvmList(j_object, PhantomData)
   }

   fn j_object(&self) -> &JObject<'local> {
      &self.0
   }

   fn into_j_object(self) -> JObject<'local> {
      self.0
   }
}

#[repr(transparent)]
pub struct JvmPair<'local, A, B>(JObject<'local>, PhantomData<(A, B)>)
where
   A: JvmType<'local>,
   B: JvmType<'local>;

impl<'local, A, B> JvmType<'local> for JvmPair<'local, A, B>
where
   A: JvmType<'local>,
   B: JvmType<'local>
{
   unsafe fn from_j_object(j_object: JObject<'local>) -> JvmPair<'local, A, B> {
      JvmPair(j_object, PhantomData)
   }

   fn j_object(&self) -> &JObject<'local> {
      &self.0
   }

   fn into_j_object(self) -> JObject<'local> {
      self.0
   }
}

#[repr(transparent)]
pub struct JvmTriple<'local, A, B, C>(JObject<'local>, PhantomData<(A, B, C)>)
where
   A: JvmType<'local>,
   B: JvmType<'local>,
   C: JvmType<'local>;

impl<'local, A, B, C> JvmType<'local> for JvmTriple<'local, A, B, C>
where
   A: JvmType<'local>,
   B: JvmType<'local>,
   C: JvmType<'local>
{
   unsafe fn from_j_object(j_object: JObject<'local>) -> JvmTriple<'local, A, B, C> {
      JvmTriple(j_object, PhantomData)
   }

   fn j_object(&self) -> &JObject<'local> {
      &self.0
   }

   fn into_j_object(self) -> JObject<'local> {
      self.0
   }
}

#[repr(transparent)]
pub struct JvmCache<'local, T: JvmType<'local>>(JObject<'local>, PhantomData<T>);

impl<'local, T: JvmType<'local>> JvmType<'local> for JvmCache<'local, T> {
   unsafe fn from_j_object(j_object: JObject<'local>) -> JvmCache<'local, T> {
      JvmCache(j_object, PhantomData)
   }

   fn j_object(&self) -> &JObject<'local> {
      &self.0
   }

   fn into_j_object(self) -> JObject<'local> {
      self.0
   }
}

#[repr(transparent)]
pub struct JvmRepository<'local, T: JvmType<'local>>(JObject<'local>, PhantomData<T>);

impl<'local, T: JvmType<'local>> JvmType<'local> for JvmRepository<'local, T> {
   unsafe fn from_j_object(j_object: JObject<'local>) -> JvmRepository<'local, T> {
      JvmRepository(j_object, PhantomData)
   }

   fn j_object(&self) -> &JObject<'local> {
      &self.0
   }

   fn into_j_object(self) -> JObject<'local> {
      self.0
   }
}

/// Kotlinファイル上でなんらかの仮型引数が当てられていて、JNI呼び出し時には
/// 型情報が消去されているもの
#[repr(transparent)]
pub struct JvmErased<'local>(
   JObject<'local>
);

impl<'local> JvmType<'local> for JvmErased<'local> {
   unsafe fn from_j_object(j_object: JObject<'local>) -> JvmErased<'local> {
      JvmErased(j_object)
   }

   fn j_object(&self) -> &JObject<'local> {
      &self.0
   }

   fn into_j_object(self) -> JObject<'local> {
      self.0
   }
}

jvm_type! {
   JvmBoolean,
   JvmByte,
   JvmShort,
   JvmInteger,
   JvmLong,
   JvmFloat,
   JvmDouble,
   JvmAny,
   JvmUnit,
   // Nothing型のインスタンスが存在することはありえないが、仮にNothing型が
   // 期待される場所になんらかのj_objectの値が存在した場合、それはJvmNothingと
   // して扱えるべきであるため、他のJvmTypeと同様の実装とする
   JvmNothing,
   JvmCacheId,
}
