using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reactive.Linq;
using System.Threading;
using Microsoft.VisualStudio.TestTools.UnitTesting;
using Pozyx.CAE.Lib;
using Pozyx.CAE.Lib.CellSpaces;
using Pozyx.CAE.Lib.Runners;

namespace Pozyx.CAE.Test
{
    [TestClass]
    public class RunnerTest
    {
        private static string _testOutputPath;

        [ClassInitialize]
        public static void Initialize(TestContext context)
        {
            Initialize(context.TestRunResultsDirectory);           
            context.AddResultFile(_testOutputPath);

            Initialize();
        }

        public static void Initialize(string testOutputPath)
        {
            _testOutputPath = testOutputPath;

            Initialize();
        }

        private static void Initialize()
        {
            //GCNotification.GCDone += (Action<int>) (gen =>
            //{
            //    if (gen == 2)
            //        Trace.WriteLine(string.Format("CAE:\tGC collect - Gen {0}", gen));
            //});
        }

        [TestMethod]
        public void TestSingleThreadCpuRunner()
        {
            TestRunner(new SingleThreadCpuRunner<BoolArrayCellSpace>());
        }

        [TestMethod]
        public void TestSingleThreadOneCoreCpuRunner()
        {
            TestRunner(new SingleThreadOneCoreCpuRunner<BoolArrayCellSpace>());
        }

        [TestMethod]
        public void TestOptimizedSingleThreadCpuRunner()
        {
            TestRunner(new OptimizedSingleThreadCpuRunner());
        }

        [TestMethod]
        public void TestOptimizedSingleThreadOneCoreCpuRunner()
        {
            TestRunner(new OptimizedSingleThreadOneCoreCpuRunner());
        }

        [TestMethod]
        public void TestTaskPerCellCpuRunner()
        {
            TestRunner(new TaskPerCellCpuRunner());
        }

        [TestMethod]
        public void TestTaskPerCellStepCpuRunner()
        {
            TestRunner(new TaskPerCellStepCpuRunner());
        }

        [TestMethod]
        public void TestPLinqPerStepCpuRunner()
        {
            TestRunner(new PLinqPerStepCpuRunner());
        }

        [TestMethod]
        public void TestTaskPerCoreStepCpuRunner()
        {
            TestRunner(new TaskPerCoreStepCpuRunner());
        }

        [TestMethod]
        public void TestThreadPoolWorkItemPerCoreStepCpuRunner()
        {
            TestRunner(new ThreadPoolWorkItemPerCoreStepCpuRunner());
        }

        [TestMethod]
        public void TestTaskPerCoreCpuRunner()
        {
            TestRunner(new TaskPerCoreCpuRunner());
        }

        [TestMethod]
        public void TestCppSingleThreadCpuRunner()
        {
            TestRunner(new CppSingleThreadCpuRunner());
        }

        [TestMethod]
        public void TestThreadPerCellStepCpuSyncedGpuRunner()
        {
            TestRunner(new ThreadPerCellStepCpuSyncedGpuRunner());
        }

        [TestMethod]
        public void TestThreadPerMultiCellsStepCpuSyncedGpuRunner()
        {
            TestRunner(new ThreadPerMultiCellsStepCpuSyncedGpuRunner());
        }

        [TestMethod]
        public void TestThreadPerCellStepCpuSyncedTiledGpuRunner()
        {
            TestRunner(new ThreadPerCellStepCpuSyncedTiledGpuRunner());
        }

        [TestMethod]
        public void TestPackedIntSingleThreadCpuRunner()
        {
            TestRunner(new SingleThreadCpuRunner<PackedIntArrayCellSpace>());
        }

        [TestMethod]
        public void TestCppSingleThreadCpuPackedIntRunner()
        {
            TestRunner(new CppSingleThreadCpuPackedIntRunner());
        }

        [TestMethod]
        public void TestThreadPerCellStepCpuSyncedGpuPackedRunner()
        {
            TestRunner(new ThreadPerCellStepCpuSyncedGpuPackedRunner());
        }

        [TestMethod]
        public void TestThreadPerCellStepCpuSyncedTiledGpuPackedRunner()
        {
            TestRunner(new ThreadPerCellStepCpuSyncedTiledGpuPackedRunner());
        }

        private void TestRunner<TCellSpace>(IRunner<TCellSpace> runner)
            where TCellSpace : ICellSpace, new()
        {
            using (runner)
            {
                //TestRunnerAndCompareWithRef(runner, 110, 5);
                TestRunner(runner, 110, 20, TestType.TraceStatistics);
            }
        }

        public void TestRunnerAndCompareWithRef<TCellSpace>(IRunner<TCellSpace> runner, int ruleNumber, int seconds)
            where TCellSpace : ICellSpace, new()
        {
            Trace.WriteLine($"CAE:\tRunning CA using {runner.GetType().Name}, rule {ruleNumber} (for {seconds} sec.)...");
            // TestType.RecordOutput |
            var result = TestRunner(runner, ruleNumber, seconds, TestType.TraceStatistics | TestType.RecordOutputToMemory);

            if (result.Count < 100)
                Trace.WriteLine("CAE:\tExecution: Incorrect (finished too early - bug suspected)");

            Assert.IsTrue(result.Count > 100);

            var refRunner = new TaskPerCoreStepCpuRunner();

            Trace.WriteLine($"CAE:\tRunning Ref. CA using {refRunner.GetType().Name}...");

            var referenceResult = TestRunner(refRunner, ruleNumber, seconds, TestType.RecordOutputToMemory);

            var csComparer = new CellSpaceTools.CellSpaceEqualityComparer();
            
            var equals = result
                .Zip(referenceResult, (r, rr) => new {Testing = r, Ref = rr})
                .All(r => csComparer.Equals(r.Testing, r.Ref));

            Trace.WriteLine($"CAE:\tExecution: {(@equals ? "OK" : "Incorrect (bug detected)")}");

            Assert.IsTrue(equals);

            Trace.WriteLine($"CAE:\tSpeedup factor (to Ref.): {(double) result.Count/referenceResult.Count:0.##}");
        }

        private static List<TCellSpace> TestRunner<TCellSpace>
            (IRunner<TCellSpace> runner, int ruleNumber, int seconds, TestType testType) 
            where TCellSpace : ICellSpace, new()
        {                                                   
            var cts = new CancellationTokenSource(TimeSpan.FromSeconds(seconds));             

            var connectableOutputObservable = runner.Create(ruleNumber, cts.Token);
            var outputObservable = (IObservable<TCellSpace>)connectableOutputObservable;

            StreamWriter statsSw = null;

            try
            {
                if (testType.HasFlag(TestType.RecordStatistics) || testType.HasFlag(TestType.TraceStatistics))
                {
                    if (testType.HasFlag(TestType.RecordStatistics))
                        statsSw = new StreamWriter(GetTestStatsFileName(runner, ruleNumber));

                    var bufferredObservable = outputObservable.Buffer(TimeSpan.FromSeconds(1));

                    long time = 0;
                    long iterations = 0;

                    if (testType.HasFlag(TestType.RecordStatistics))
                        statsSw.WriteLine("Time,Iterations,Width");

                    bufferredObservable.Subscribe(bufItems =>
                    {
                        time++;
                        iterations += bufItems.Count;

                        if (testType.HasFlag(TestType.RecordStatistics))
                        {
                            statsSw.WriteLine("{0},{1},{2}",
                                time, iterations,
                                bufItems.Any() ? bufItems.Last().Length.ToString(CultureInfo.InvariantCulture) : "N/A");
                            statsSw.Flush();
                        }
                        if (testType.HasFlag(TestType.TraceStatistics))
                        {
                            Trace.WriteLine(
                                $"CAE:\tT+{time}\tIterations: {iterations}\tWidth: {(bufItems.Any() ? bufItems.Last().Length.ToString(CultureInfo.InvariantCulture) : "N/A")}");
                        }
                    }, ex => { }); // because subsequent subscriptions would not receive error without this

                    outputObservable = bufferredObservable.SelectMany(b => b);
                }

                List<TCellSpace> outputList = null;

                if (testType.HasFlag(TestType.RecordOutput) || testType.HasFlag(TestType.RecordOutputToMemory))
                {
                    outputList = new List<TCellSpace>();

                    outputObservable.Subscribe(
                        item => outputList.Add(item),
                        ex => { }); // because subsequent subscriptions would not receive error without this
                }

                connectableOutputObservable.Connect();

                // because don't know how to wait (block) for empty observable
                outputObservable.Concat(Observable.Return(default(TCellSpace))).Wait();

                if (testType.HasFlag(TestType.RecordOutput))
                {
                    var minOffset = outputList.Min(o => o.Offset);

                    using (var sw = new StreamWriter(GetTestOutputFileName(runner, ruleNumber)))
                        foreach (var item in outputList)
                            sw.WriteLine(item.ToCellString(minOffset));
                }

                return testType.HasFlag(TestType.RecordOutputToMemory) ? outputList : null;
            }
            catch (Exception ex)
            {
                var exceptionString = ex.ToString();

                Trace.WriteLine(string.Join(Environment.NewLine, 
                        exceptionString.Split(new[] { Environment.NewLine }, StringSplitOptions.None)
                            .Select(l => $"CAE:\t{l}")));

                Console.WriteLine(exceptionString);

                return null;
            }
            finally
            {                
                if (testType.HasFlag(TestType.RecordStatistics))
                    statsSw.Dispose();
            }                  
        }

        private static string GetTestOutputFileName<TCellSpace>
            (IRunner<TCellSpace> runner, int ruleNumber) where TCellSpace : ICellSpace, new()
        {
            if (!Directory.Exists(_testOutputPath))
                Directory.CreateDirectory(_testOutputPath);

            return Path.Combine(
                _testOutputPath,
                $"Test output for {runner.GetType().Name} rule {ruleNumber}.txt");
        }

        private static string GetTestStatsFileName<TCellSpace>
            (IRunner<TCellSpace> runner, int ruleNumber) where TCellSpace : ICellSpace, new()
        {
            if (!Directory.Exists(_testOutputPath))
                Directory.CreateDirectory(_testOutputPath);

            return Path.Combine(
                _testOutputPath,
                $"Test stats for {runner.GetType().Name} rule {ruleNumber}.csv");
        }        

        [Flags]
        enum TestType
        {
            None = 0,
            RecordOutput = 1,
            RecordStatistics = 2,
            TraceStatistics = 4,
            RecordOutputToMemory = 8,
            RecordAll = RecordOutput | RecordStatistics | TraceStatistics | RecordOutputToMemory
        }
    }    
}