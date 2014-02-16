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

            //(new RunnerTest()).TestSingleThreadCpuRunner();
            //(new RunnerTest()).TestSingleThreadOneCoreCpuRunner();
            //(new RunnerTest()).TestTaskPerCellCpuRunner();
            //(new RunnerTest()).TestTaskPerCellStepCpuRunner();
            //(new RunnerTest()).TestPLinqPerStepCpuRunner();     
            //(new RunnerTest()).TestTaskPerCoreStepCpuRunner();
            (new RunnerTest()).TestThreadPoolWorkItemPerCoreStepCpuRunner();
        }        
    }
}
