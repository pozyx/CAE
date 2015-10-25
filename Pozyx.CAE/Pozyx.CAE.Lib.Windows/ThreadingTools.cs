using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Threading;

namespace Pozyx.CAE.Lib
{
    static class ThreadingTools
    {        
        /// <summary>
        /// Sets the processor affinity of the current thread.
        /// </summary>
        /// <param name="cpus">A list of CPU numbers. The values should be
        /// between 0 and <see cref="Environment.ProcessorCount"/>.</param>
        /// <remarks>Taken from http://stackoverflow.com/questions/12328751/set-thread-processor-affinity-in-microsoft-net</remarks>
        public static void SetThreadProcessorAffinity(params int[] cpus)
        {
            if (cpus == null)
                throw new ArgumentNullException(nameof(cpus));
            if (cpus.Length == 0)
                throw new ArgumentException("You must specify at least one CPU.", nameof(cpus));            

            // Supports up to 64 processors
            long cpuMask = 0;
            foreach (var cpu in cpus)
            {
                if (cpu < 0 || cpu >= Environment.ProcessorCount)
                    throw new ArgumentException("Invalid CPU number.");

                cpuMask |= 1L << cpu;
            }            

            // Ensure managed thread is linked to OS thread; does nothing on default host in current .Net versions
            Thread.BeginThreadAffinity();

#pragma warning disable 618
            // The call to BeginThreadAffinity guarantees stable results for GetCurrentThreadId,
            // so we ignore the obsolete warning
            var osThreadId = AppDomain.GetCurrentThreadId();
#pragma warning restore 618

            // Find the ProcessThread for this thread.
            var thread = Process.GetCurrentProcess().Threads.Cast<ProcessThread>().Single(t => t.Id == osThreadId);
            
            // Set the thread's processor affinity
            thread.ProcessorAffinity = new IntPtr(cpuMask);
        }

        //public static void StartThreadPoolWorkItemsAndWait(IList<Action> actions)
        //{
        //    var waitHandles = new WaitHandle[actions.Count];

        //    for (var i = 0; i < actions.Count; i++)
        //    {
        //        var manualResetEvent = new ManualResetEventSlim(false);

        //        waitHandles[i] = manualResetEvent.WaitHandle;

        //        var iCaptured = i;

        //        ThreadPool.QueueUserWorkItem(_ =>
        //        {
        //            try
        //            {
        //                actions[iCaptured]();
        //            }
        //            finally
        //            {
        //                manualResetEvent.Set();
        //            } 
        //        });
        //    }

        //    WaitHandle.WaitAll(waitHandles);
        //}

        public static void StartThreadPoolWorkItemsAndWait(IList<Action> actions)
        {
            var counter = actions.Count;

            for (var i = 0; i < actions.Count - 1; i++)
            {
                var iCaptured = i;

                ThreadPool.QueueUserWorkItem(_ =>
                {
                    try
                    {
                        actions[iCaptured]();
                    }
                    finally
                    {
                        Interlocked.Decrement(ref counter);
                    }
                });
            }

            try
            {
                actions[actions.Count - 1]();
            }
            finally
            {
                Interlocked.Decrement(ref counter);
            }

            //SpinWait.SpinUntil(() =>
            //{
            //    var counterValue = Volatile.Read(ref counter);
            //    return counterValue == 0;
            //});

            while (counter != 0) 
                Thread.MemoryBarrier();
        }
    }
}
