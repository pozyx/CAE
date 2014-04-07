﻿using Pozyx.CAE.Test;
using System;
using System.IO;

namespace Pozyx.CAE.TestApp
{
    class Program
    {
        static void Main()
        {
            RunnerTest.Initialize(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory), "Pozyx.CAE"));

            // (new RunnerTest()).TestSingleThreadCpuRunner();
            // (new RunnerTest()).TestSingleThreadOneCoreCpuRunner();
            // (new RunnerTest()).TestOptimizedSingleThreadCpuRunner();
            // (new RunnerTest()).TestOptimizedSingleThreadOneCoreCpuRunner();
            // (new RunnerTest()).TestTaskPerCellCpuRunner();
            // (new RunnerTest()).TestTaskPerCellStepCpuRunner();
            // (new RunnerTest()).TestPLinqPerStepCpuRunner();     
            // (new RunnerTest()).TestTaskPerCoreStepCpuRunner();
            // (new RunnerTest()).TestThreadPoolWorkItemPerCoreStepCpuRunner();
            (new RunnerTest()).TestTaskPerCoreCpuRunner();
            // (new RunnerTest()).TestThreadPerCellStepCpuSyncedGpuRunner();

            //Lib.Windows.Test.RunTest();

            //Console.ReadLine();
        }        
    }
}
