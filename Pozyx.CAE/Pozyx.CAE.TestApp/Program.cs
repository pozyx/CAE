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

            // 72k (20s)
            // (new RunnerTest()).TestOptimizedSingleThreadCpuRunner(); // REF
            // (new RunnerTest()).TestOptimizedSingleThreadOneCoreCpuRunner();

            // 1.2k (20s)
            // (new RunnerTest()).TestTaskPerCellCpuRunner();

            // 11k (20s)
            // (new RunnerTest()).TestTaskPerCellStepCpuRunner();

            // 71k (20s)
            // (new RunnerTest()).TestPLinqPerStepCpuRunner();

            // 115k (20s)
            // (new RunnerTest()).TestTaskPerCoreStepCpuRunner();
            // (new RunnerTest()).TestThreadPoolWorkItemPerCoreStepCpuRunner();

            // 115k (20s)
            // (new RunnerTest()).TestTaskPerCoreCpuRunner();            

            // 86k (20s)
            //(new RunnerTest()).TestCppSingleThreadCpuRunner();

            // GPU performance depends on drivers, some are like 4x slower!

            // 40k (20s)
            // (new RunnerTest()).TestThreadPerCellStepCpuSyncedGpuRunner();

            // 34k (20s)
            // (new RunnerTest()).TestThreadPerMultiCellsStepCpuSyncedGpuRunner();

            // 48k, 38k now (20s)
            // (new RunnerTest()).TestThreadPerCellStepCpuSyncedTiledGpuRunner();

            // 39k (20s)
            //(new RunnerTest()).TestPackedIntSingleThreadCpuRunner();           

            // 82k (20s)
            //(new RunnerTest()).TestCppSingleThreadCpuPackedIntRunner();            

            // 60k (20s)
            // (new RunnerTest()).TestThreadPerCellStepCpuSyncedGpuPackedRunner();

            // 52k (20s)
            //(new RunnerTest()).TestThreadPerCellStepCpuSyncedTiledGpuPackedRunner();
        }
    }
}
