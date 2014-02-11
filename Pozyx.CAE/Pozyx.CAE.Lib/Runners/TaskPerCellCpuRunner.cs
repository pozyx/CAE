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
    // freezes

    public class TaskPerCellCpuRunner : IRunner<BoolArrayCellSpace>
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
            var beginManualResetEvent = new ManualResetEventSlim(false, 50);
            var endBarrier = new Barrier(1, _ => beginManualResetEvent.Reset());

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
                nextStep.Initialize(nextStepLength, nextStepOffset);

                for (var index = nextStepOffset; index < nextStepOffset + nextStepLength; index++)
                {
                    if (!cellTasks.ContainsKey(index))
                    {
                        endBarrier.AddParticipant();

                        var indexCaptured = index;

                        var cellTask = new Task(() =>
                        {
                            while (true)
                            {
                                finishCellsCt.ThrowIfCancellationRequested();

                                beginManualResetEvent.Wait(finishCellsCt);

                                RuleTools.ApplyRule(prevStep, nextStep, indexCaptured, rule);

                                endBarrier.SignalAndWait(finishCellsCt);
                            }

                        }, finishCellsCt, TaskCreationOptions.LongRunning | TaskCreationOptions.AttachedToParent);

                        cellTask.ContinueWith(_ => endBarrier.RemoveParticipant());

                        cellTasks.Add(index, cellTask);

                        cellTask.Start();
                    }
                }

                beginManualResetEvent.Set();
                endBarrier.SignalAndWait();

                observer.OnNext(nextStep);

                CellSpaceTools.GetChangeBounds(prevStep, nextStep, out leftMostChangedIndex, out rightMostChangedIndex);

                prevStep = nextStep;
            }
        }
    }
}
