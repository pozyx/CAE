using Pozyx.CAE.Lib.CellSpaces;
using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Reactive.Disposables;
using System.Reactive.Linq;
using System.Reactive.Subjects;
using System.Threading;
using System.Threading.Tasks;

namespace Pozyx.CAE.Lib.Runners
{
    public class TaskPerCoreCpuRunner : IRunner<BoolArrayCellSpace>
    {
        public IConnectableObservable<BoolArrayCellSpace> Create(int ruleNumber, CancellationToken ct)
        {
            var rule = RuleTools.GetBoolArrayForRule(ruleNumber);

            return Observable.Create<BoolArrayCellSpace>(observer =>
            {
                var finishCellsCts = new CancellationTokenSource();

                Task.Run(() => Run(observer, rule, ct, finishCellsCts.Token), ct)
                .ContinueWith(t =>
                {
                    if (t.IsCanceled)
                        observer.OnCompleted();
                    else
                        observer.OnError(t.Exception);
                },
                TaskContinuationOptions.NotOnRanToCompletion)
                .ContinueWith(_ => finishCellsCts.Cancel());

                return Disposable.Empty;
            })
            .Publish();
        }

        private static void Run(
            IObserver<BoolArrayCellSpace> observer, 
            bool[] rule, 
            CancellationToken ct,
            CancellationToken finishCellsCt)
        {
            var beginManualResetEvent = new ManualResetEvent(false);
            var endBarrier = new Barrier(1 + Environment.ProcessorCount, _ => beginManualResetEvent.Reset());

            var cellTasks = new List<Task>(Environment.ProcessorCount);

            var prevStep = new BoolArrayCellSpace();
            prevStep.Initialize(new BitArray(1, true), 0);
            observer.OnNext(prevStep);
            BoolArrayCellSpace nextStep = null;

            int? leftMostChangedIndex = 0;
            int? rightMostChangedIndex = 0;

            var iterationsPerCore = -1;
            var nextStepLength = -1;
            var nextStepOffset = -1;

            for (var i = 0; i < Environment.ProcessorCount; i++)
            {
                var iCaptured = i;

                // no performance improvement when reusing main thread for work
                var cellTask = new Task(() =>
                {
                    while (true)
                    {
                        finishCellsCt.ThrowIfCancellationRequested();

                        beginManualResetEvent.WaitOne();

                        var startIndex = nextStepOffset + (iCaptured * iterationsPerCore);

                        var endIndex =
                            iCaptured == Environment.ProcessorCount - 1 ?
                            nextStepOffset + nextStepLength :
                            startIndex + iterationsPerCore;

                        RuleTools.ApplyRule(prevStep, nextStep, rule, startIndex - nextStepOffset, endIndex - nextStepOffset);

                        endBarrier.SignalAndWait(finishCellsCt);
                    }

                }, finishCellsCt, TaskCreationOptions.LongRunning | TaskCreationOptions.AttachedToParent);

                cellTask.ContinueWith(_ => endBarrier.RemoveParticipant());

                cellTasks.Add(cellTask);

                cellTask.Start();
            }

            while (true)
            {
                ct.ThrowIfCancellationRequested();

                var faultedTasks = cellTasks.Where(t => t.IsFaulted);

                if (faultedTasks.Any())
                    throw new AggregateException(faultedTasks.Select(t => t.Exception));

                if (!leftMostChangedIndex.HasValue)
                {
                    observer.OnCompleted();
                    break;
                }

                nextStepLength = rightMostChangedIndex.Value - leftMostChangedIndex.Value + 3;
                nextStepOffset = leftMostChangedIndex.Value - 1;

                nextStep = new BoolArrayCellSpace();
                nextStep.Initialize(nextStepLength, nextStepOffset);             

                iterationsPerCore = nextStepLength / Environment.ProcessorCount;

                beginManualResetEvent.Set();
                endBarrier.SignalAndWait();

                observer.OnNext(nextStep);

                CellSpaceTools.GetChangeBounds(prevStep, nextStep, out leftMostChangedIndex, out rightMostChangedIndex);

                prevStep = nextStep;
            }
        }
    }
}
