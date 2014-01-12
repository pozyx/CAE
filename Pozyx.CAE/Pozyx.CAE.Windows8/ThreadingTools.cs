using System;
using System.Diagnostics;
using System.Linq;
using System.Threading;

namespace Pozyx.CAE.Windows8
{
    public static class ThreadingTools
    {        
        /// <summary>
        /// Sets the processor affinity of the current thread.
        /// </summary>
        /// <param name="cpus">A list of CPU numbers. The values should be
        /// between 0 and <see cref="Environment.ProcessorCount"/>.</param>
        public static void SetThreadProcessorAffinity(params int[] cpus)
        {
            if (cpus == null)
                throw new ArgumentNullException("cpus");
            if (cpus.Length == 0)
                throw new ArgumentException("You must specify at least one CPU.", "cpus");            

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
    }
}
