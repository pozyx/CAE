using System.Diagnostics;
using Pozyx.CAE.Test;
using System;
using System.IO;

namespace Pozyx.CAE.TestApp
{
    class Program
    {
        static void Main()
        {
            Trace.Listeners.Add(new ConsoleTraceListener());
            RunnerTest.Initialize(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), "Pozyx.CAE"));

            // 47k (20s)
            // (new RunnerTest()).TestSingleThreadCpuRunner();
            // (new RunnerTest()).TestSingleThreadOneCoreCpuRunner();

            // 76k (20s)
            // (new RunnerTest()).TestOptimizedSingleThreadCpuRunner(); // REF
            // (new RunnerTest()).TestOptimizedSingleThreadOneCoreCpuRunner();

            // 1k (20s)
            // (new RunnerTest()).TestTaskPerCellCpuRunner();

            // 11k (20s)
            //(new RunnerTest()).TestTaskPerCellStepCpuRunner();
            
            // 71k (20s)
            //(new RunnerTest()).TestPLinqPerStepCpuRunner();

            // 118k (20s)
            // (new RunnerTest()).TestTaskPerCoreStepCpuRunner();
            // (new RunnerTest()).TestThreadPoolWorkItemPerCoreStepCpuRunner();

            // 125k (20s), 523k (300s)
            // (new RunnerTest()).TestTaskPerCoreCpuRunner();            

            // 86k (20s), 323k (300s)
            //(new RunnerTest()).TestCppSingleThreadCpuRunner();

            // GPU performance depends on drivers, now 340.52 (quite ok), 344.75 - 4x slower!

            // 33k (20s) - but depends on drivers (it was different 8-43), 296k (300s)
            (new RunnerTest()).TestThreadPerCellStepCpuSyncedGpuRunner();

            // 21k (20s) - but depends on drivers (it was different ?-33), 207k (300s)
            //(new RunnerTest()).TestThreadPerMultiCellsStepCpuSyncedGpuRunner();

            //(new RunnerTest()).TestThreadPerCellStepCpuSyncedTiledGpuRunner();
        }        
    }
}
