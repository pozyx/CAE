using Pozyx.CAE.Test;
using System;
using System.IO;

namespace Pozyx.CAE.TestApp
{
    class Program
    {
        static void Main()
        {
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

            // 43k - why now 26 and now 33 and now 8??? (20s), 296k (300s)
            (new RunnerTest()).TestThreadPerCellStepCpuSyncedGpuRunner();

            // 32k - why now 17??? (20s), 207k (300s)
            // (new RunnerTest()).TestThreadPerMultiCellsStepCpuSyncedGpuRunner();

            //(new RunnerTest()).TestThreadPerCellStepCpuSyncedTiledGpuRunner();
        }        
    }
}
