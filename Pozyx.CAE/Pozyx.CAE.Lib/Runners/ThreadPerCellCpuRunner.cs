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
    // ManualResetEventSlim - freezes after cca 30 sec.
    // ManualResetEvent - freezes after cca 3 sec.

    public class ThreadPerCellCpuRunner : IRunner<BoolArrayCellSpace>
    {
        public IConnectableObservable<BoolArrayCellSpace> Create(int ruleNumber, CancellationToken ct, Action threadInit = null)
        {
            var rule = RulesTools.GetBitArrayForRule(ruleNumber);

            return Observable.Create<BoolArrayCellSpace>(observer =>
            {
                var finishCellsCts = new CancellationTokenSource();

                Task.Run(() => Run(observer, rule, ct, finishCellsCts.Token, threadInit), ct)
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
            BitArray rule,
            CancellationToken ct,
            CancellationToken finishCellsCt,
            Action threadInit)
        {
            var beginManualResetEvent = new ManualResetEventSlim(false, 50);
            var endBarrier = new Barrier(1, _ => beginManualResetEvent.Reset());
            var boundsSyncObj = new object();

            var cellTasks = new Dictionary<int, Task>();

            var prevStep = new BoolArrayCellSpace();
            prevStep.Initialize(new BitArray(1, true), 0);
            observer.OnNext(prevStep);
            BoolArrayCellSpace nextStep;

            int? leftMostChangedIndex = 0;
            int? rightMostChangedIndex = 0;

            while (true)
            {
                ct.ThrowIfCancellationRequested();

                var faultedTasks = cellTasks.Values.Where(t => t.IsFaulted).ToList();

                if (faultedTasks.Any())
                    throw new AggregateException(faultedTasks.Select(t => t.Exception));

                if (!leftMostChangedIndex.HasValue)
                {
                    observer.OnCompleted();
                    break;
                }

                var nextStepLength = rightMostChangedIndex.Value - leftMostChangedIndex.Value + 3;
                var nextStepOffset = leftMostChangedIndex.Value - 1;

                nextStep = new BoolArrayCellSpace();
                nextStep.Initialize(new BitArray(nextStepLength), nextStepOffset);

                leftMostChangedIndex = null;
                rightMostChangedIndex = null;

                for (var index = nextStepOffset; index < nextStepOffset + nextStepLength; index++)
                {
                    if (!cellTasks.ContainsKey(index))
                    {
                        endBarrier.AddParticipant();

                        var indexCaptured = index;

                        var cellTask = new Task(() =>
                        {
                            if (threadInit != null)
                                threadInit();

                            RunCell(
                                ref prevStep,
                                ref nextStep,
                                indexCaptured,
                                rule,
                                ref leftMostChangedIndex,
                                ref rightMostChangedIndex,
                                finishCellsCt,
                                beginManualResetEvent,
                                endBarrier,
                                boundsSyncObj);

                        }, finishCellsCt, TaskCreationOptions.LongRunning | TaskCreationOptions.AttachedToParent);

                        cellTask.ContinueWith(_ => endBarrier.RemoveParticipant());

                        cellTasks.Add(index, cellTask);

                        cellTask.Start();
                    }
                }

                beginManualResetEvent.Set();
                endBarrier.SignalAndWait();

                observer.OnNext(nextStep);

                prevStep = nextStep;
            }
        }

        private static void RunCell(
            ref BoolArrayCellSpace prevStep,
            ref BoolArrayCellSpace nextStep,
            int index,
            BitArray rule,
            ref int? leftMostChangedIndex,
            ref int? rightMostChangedIndex,
            CancellationToken finishCellsCt,
            ManualResetEventSlim beginManualResetEvent,
            Barrier endBarrier,
            object boundsSyncObj)
        {
            while (true)
            {
                finishCellsCt.ThrowIfCancellationRequested();

                beginManualResetEvent.Wait(finishCellsCt);

                var trueOrChanged = RulesTools.ApplyRule(prevStep, nextStep, index, rule);

                if (trueOrChanged)
                {
                    lock (boundsSyncObj)
                    {
                        if (!leftMostChangedIndex.HasValue || index < leftMostChangedIndex.Value)
                            leftMostChangedIndex = index;

                        if (!rightMostChangedIndex.HasValue || index > rightMostChangedIndex.Value)
                            rightMostChangedIndex = index;
                    }
                }
                else
                {
                    // can be improved: finish task when not needed
                }

                endBarrier.SignalAndWait(finishCellsCt);
            }
        }
    }
}
