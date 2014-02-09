using Microsoft.VisualStudio.TestTools.UnitTesting;
using Pozyx.CAE.Lib.CellSpaces;
using Pozyx.CAE.Lib.Runners;
using Pozyx.CAE.Windows8;
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reactive.Linq;
using System.Threading;

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
        }

        public static void Initialize(string testOutputPath)
        {
            _testOutputPath = testOutputPath;            
        }

        //[TestMethod]
        public void TestSingleThreadCpuRunner()
        {
            TestRunner(new SingleThreadCpuRunner<BoolArrayCellSpace>(), 110, 20, TestType.TraceStatistics, false);
        }

        [TestMethod]
        public void TestThreadPerCellCpuRunner()
        {
            TestRunnerAndCompareWithRef(new ThreadPerCellCpuRunner(), 110, 5);
        }

        public void TestRunnerAndCompareWithRef<TCellSpace>(IRunner<TCellSpace> runner, int ruleNumber, int seconds)
            where TCellSpace : ICellSpace, new()
        {
            Trace.WriteLine("CAE\tRunning...");
            // TestType.RecordOutput |
            var result = TestRunner(runner, ruleNumber, seconds, TestType.TraceStatistics | TestType.RecordOutputToMemory);

            if (result.Count < 100)
                Trace.WriteLine(string.Format("CAE\tExecution: Incorrect (finished too early)"));

            Assert.IsTrue(result.Count > 100);

            Trace.WriteLine("CAE\tRunning (Ref.)...");
            var referenceResult = TestRunner(new SingleThreadCpuRunner<BoolArrayCellSpace>(), ruleNumber, seconds, TestType.RecordOutputToMemory);

            var csComparer = new CellSpaceEqualityComparer();
            
            var equals = result
                .Zip(referenceResult, (r, rr) => new {Testing = r, Ref = rr})
                .All(r => csComparer.Equals(r.Testing, r.Ref));

            Trace.WriteLine(string.Format("CAE\tExecution: {0}", equals ? "OK" : "Incorrect"));

            Assert.IsTrue(equals);

            Trace.WriteLine(string.Format("CAE\tSpeedup factor (to Ref.): {0:0.##}", (double) result.Count / referenceResult.Count));
        }

        private static List<TCellSpace> TestRunner<TCellSpace>
            (IRunner<TCellSpace> runner, int ruleNumber, int seconds, TestType testType, bool setThreadAffinity = false) 
            where TCellSpace : ICellSpace, new()
        {                                                   
            var cts = new CancellationTokenSource(TimeSpan.FromSeconds(seconds));

            Action threadInit = null;

            if (setThreadAffinity)            
                threadInit = () => ThreadingTools.SetThreadProcessorAffinity(1);                            

            var connectableOutputObservable = runner.Create(ruleNumber, cts.Token, threadInit);
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
                            Trace.WriteLine(string.Format("CAE\tT+{0}\tIterations: {1}\tWidth: {2}",
                                time, iterations,
                                bufItems.Any() ? bufItems.Last().Length.ToString(CultureInfo.InvariantCulture) : "N/A"));
                        }
                    });

                    outputObservable = bufferredObservable.SelectMany(b => b);
                }

                List<TCellSpace> outputList = null;

                if (testType.HasFlag(TestType.RecordOutput) || testType.HasFlag(TestType.RecordOutputToMemory))
                {
                    outputList = new List<TCellSpace>();

                    outputObservable.Subscribe(item => outputList.Add(item));
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
                            .Select(l => string.Format("CAE\t{0}", l))));

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
                string.Format("Test output for {0} rule {1}.txt", runner.GetType().Name, ruleNumber));
        }

        private static string GetTestStatsFileName<TCellSpace>
            (IRunner<TCellSpace> runner, int ruleNumber) where TCellSpace : ICellSpace, new()
        {
            if (!Directory.Exists(_testOutputPath))
                Directory.CreateDirectory(_testOutputPath);

            return Path.Combine(
                _testOutputPath,
                string.Format("Test stats for {0} rule {1}.csv", runner.GetType().Name, ruleNumber));
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