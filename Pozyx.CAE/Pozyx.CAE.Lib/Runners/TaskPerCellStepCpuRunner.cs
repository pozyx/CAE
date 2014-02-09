using Pozyx.CAE.Lib.CellSpaces;
using System;
using System.Collections;
using System.Collections.Generic;
using System.Reactive.Disposables;
using System.Reactive.Linq;
using System.Reactive.Subjects;
using System.Threading;
using System.Threading.Tasks;

namespace Pozyx.CAE.Lib.Runners
{
    public class TaskPerCellStepCpuRunner : IRunner<BoolArrayCellSpace>
    {
        public IConnectableObservable<BoolArrayCellSpace> Create(int ruleNumber, CancellationToken ct, Action threadInit = null)
        {
            var rule = RulesTools.GetBitArrayForRule(ruleNumber);

            return Observable.Create<BoolArrayCellSpace>(observer =>
            {
                Task.Run(() => Run(observer, rule, ct, threadInit), ct)
                .ContinueWith(t =>
                {
                    if (t.IsCanceled)
                        observer.OnCompleted();
                    else
                        observer.OnError(t.Exception);
                },
                TaskContinuationOptions.NotOnRanToCompletion);

                return Disposable.Empty;
            })
            .Publish();
        }

        private static void Run(
            IObserver<BoolArrayCellSpace> observer,
            BitArray rule,
            CancellationToken ct,
            Action threadInit)
        {
            var boundsSyncObj = new object();

            var prevStep = new BoolArrayCellSpace();
            prevStep.Initialize(new BitArray(1, true), 0);
            observer.OnNext(prevStep);
            BoolArrayCellSpace nextStep;

            int? leftMostChangedIndex = 0;
            int? rightMostChangedIndex = 0;

            while (true)
            {
                ct.ThrowIfCancellationRequested();

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

                var cellTasksForStep = new List<Task>();

                for (var index = nextStepOffset; index < nextStepOffset + nextStepLength; index++)
                {
                    var indexCaptured = index;

                    var cellTask = new Task(() =>
                    {
                        if (threadInit != null)
                            threadInit();

                        RunCellStep(
                            ref prevStep,
                            ref nextStep,
                            indexCaptured,
                            rule,
                            ref leftMostChangedIndex,
                            ref rightMostChangedIndex,
                            boundsSyncObj);

                    }, TaskCreationOptions.AttachedToParent);

                    cellTasksForStep.Add(cellTask);

                    cellTask.Start();
                }

                Task.WaitAll(cellTasksForStep.ToArray());

                observer.OnNext(nextStep);

                prevStep = nextStep;
            }
        }

        private static void RunCellStep(
            ref BoolArrayCellSpace prevStep,
            ref BoolArrayCellSpace nextStep,
            int index,
            BitArray rule,
            ref int? leftMostChangedIndex,
            ref int? rightMostChangedIndex,
            object boundsSyncObj)
        {
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
        }
    }
}
