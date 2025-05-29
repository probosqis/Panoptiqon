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

use std::collections::VecDeque;
use std::ptr;
use std::sync::atomic::AtomicPtr;
use std::sync::mpsc::Sender;
use std::sync::Mutex;
use std::thread::JoinHandle;
use crate::db::save_task::SaveTask;

static SINGLETON: DbScheduler = DbScheduler::new();

pub(crate) struct DbScheduler {
   tasks: Mutex<VecDeque<SaveTask>>,
   worker_thread: Mutex<Option<WorkerThread>>,
   worker_thread_message_sender: AtomicPtr<Sender<WorkerThreadMessage>>
}

impl DbScheduler {
   pub(crate) fn singleton() -> &'static Self {
      &SINGLETON
   }

   pub(crate) const fn new() -> Self {
      Self {
         tasks: Mutex::new(VecDeque::new()),
         worker_thread: Mutex::new(None),
         worker_thread_message_sender: AtomicPtr::new(ptr::null_mut())
      }
   }

   /// WorkerThreadにメッセージを送信するためのSenderを取得する。
   ///
   /// WorkerThreadが起動されていない場合、**WorkerThreadを起動して**
   /// そのSenderを返却する。
   fn message_sender(&self) -> &Sender<WorkerThreadMessage> {
      use std::sync::atomic::Ordering;

      let sender = unsafe {
         self.worker_thread_message_sender.load(Ordering::Relaxed).as_ref()
      };

      // worker_thread_message_senderが非nullの場合。
      // ロックなしで返却可能
      if let Some(sender) = sender { return sender; }

      // worker_thread_message_senderがnullの場合。
      // ロックを取得
      let mut lock = self.worker_thread.lock().unwrap();

      let message_sender_ptr = match *lock {
         Some(ref mut worker_thread) => {
            // WorkerThreadが存在している。
            // worker_thread_message_senderがnullでも
            // 同時に別スレッドがWorkerThreadを起動していた場合は
            // ロックが取れたあとにWorkerThreadが存在することはありうる
            &mut worker_thread.message as *mut _
         }
         None => {
            // WorkerThreadが存在しない。起動する
            let mut worker_thread = WorkerThread::start();

            // worker_threadとworker_thread_message_senderに
            // 起動したインスタンスを格納
            let message_sender_ptr = &mut worker_thread.message as *mut _;
            *lock = Some(worker_thread);
            self.worker_thread_message_sender
               .store(message_sender_ptr, Ordering::Relaxed);

            message_sender_ptr
         }
      };

      // ロックを解除（タイミングを明確にするため明示）
      drop(lock);

      unsafe { &*message_sender_ptr }
   }

   pub(crate) fn push(&self, task: SaveTask) {
      self.message_sender();
      self.tasks.lock().unwrap().push_front(task);
   }

   pub(crate) fn push_singleton(task: SaveTask) {
      SINGLETON.push(task);
   }

   #[cfg(any(test, feature = "jni-test"))]
   pub(crate) fn clear_all_tasks(&self) {
      self.tasks.lock().unwrap().clear();
   }

   #[cfg(any(test, feature = "jni-test"))]
   pub(crate) fn tasks(&self) -> VecDeque<SaveTask> {
      self.tasks.lock().unwrap().clone()
   }

   #[cfg(any(test, feature = "jni-test"))]
   pub(crate) fn kill_worker_thread(&self) {
      use std::sync::atomic::Ordering;

      if let Some(worker_thread) = self.worker_thread.lock().unwrap().take() {
         worker_thread.stop();
         self.worker_thread_message_sender
            .store(ptr::null_mut(), Ordering::Relaxed);
      }
   }
}

impl Drop for DbScheduler {
   fn drop(&mut self) {
      use std::sync::atomic::Ordering;

      if let Ok(mut worker_thread) = self.worker_thread.lock() {
         if let Some(worker_thread) = worker_thread.take() {
            worker_thread.stop();

            self.worker_thread_message_sender
               .store(ptr::null_mut(), Ordering::Relaxed);
         }
      }
   }
}

struct WorkerThread {
   handle: JoinHandle<()>,
   message: Sender<WorkerThreadMessage>
}

enum WorkerThreadMessage {
   Stop,
}

impl WorkerThread {
   fn start() -> Self {
      use std::sync::mpsc;
      use std::thread;

      let (tx, rx) = mpsc::channel();

      let handle = thread::spawn(move || loop {
         let Ok(message) = rx.recv() else { break; };

         match message {
            WorkerThreadMessage::Stop => break
         }
      });

      Self {
         handle,
         message: tx
      }
   }

   fn stop(self) {
      self.message.send(WorkerThreadMessage::Stop).unwrap();
      self.handle.join().unwrap();
   }
}

#[cfg(test)]
mod test {
   use crate::db::save_task::SaveTask;
   use super::DbScheduler;

   #[allow(non_snake_case)]
   #[test]
   fn push_startWorkerThread() {
      use std::sync::atomic::Ordering;

      let db_scheduler = DbScheduler::new();

      assert!(
         db_scheduler.worker_thread.lock().unwrap()
            .is_none()
      );
      assert!(
         db_scheduler.worker_thread_message_sender.load(Ordering::Relaxed)
            .is_null()
      );

      db_scheduler.push(SaveTask::new("DbSchedulerTest/push_startWorkerThread"));

      assert!(
         db_scheduler.worker_thread.lock().unwrap()
            .is_some()
      );
      assert!(
         !db_scheduler.worker_thread_message_sender.load(Ordering::Relaxed)
            .is_null()
      );
   }
}
