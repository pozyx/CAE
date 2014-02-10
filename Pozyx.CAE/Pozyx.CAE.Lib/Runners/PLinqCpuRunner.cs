﻿using Pozyx.CAE.Lib.CellSpaces;
using System;
using System.Collections;
using System.Linq;
using System.Reactive.Disposables;
using System.Reactive.Linq;
using System.Reactive.Subjects;
using System.Threading;
using System.Threading.Tasks;

namespace Pozyx.CAE.Lib.Runners
{
    public class PLinqCpuRunner : IRunner<BoolArrayCellSpace>
    {
        public IConnectableObservable<BoolArrayCellSpace> Create(int ruleNumber, CancellationToken ct, Action threadInit = null)
        {
            if (threadInit != null)
                throw new NotSupportedException("threadInit not supported");

            var rule = RulesTools.GetBoolArrayForRule(ruleNumber);

            return Observable.Create<BoolArrayCellSpace>(observer =>
            {
                Task.Run(() => Run(observer, rule, ct), ct)
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
            bool[] rule,
            CancellationToken ct)
        {
            //var boundsSyncObj = new object();

            var prevStep = new BoolArrayCellSpace();
            prevStep.Initialize(new BitArray(1, true), 0);
            observer.OnNext(prevStep);
            BoolArrayCellSpace nextStep;

            int? leftMostChangedIndex = 0;
            int? rightMostChangedIndex = 0;

            while (true)
            {
                ct.ThrowIfCancellationRequested();

                //if (!leftMostChangedIndex.HasValue)
                //{
                //    observer.OnCompleted();
                //    break;
                //}

                var nextStepLength = rightMostChangedIndex.Value - leftMostChangedIndex.Value + 3;
                var nextStepOffset = leftMostChangedIndex.Value - 1;

                nextStep = new BoolArrayCellSpace();
                nextStep.Initialize(nextStepLength, nextStepOffset);

                //leftMostChangedIndex = null;
                //rightMostChangedIndex = null;

                ParallelEnumerable
                    .Range(nextStepOffset, nextStepLength - 1)
                    .WithCancellation(ct)
                    .ForAll(index =>
                    {
                        //RunCellStep(
                        //   ref prevStep,
                        //   ref nextStep,
                        //   index,
                        //   rule,
                        //   ref leftMostChangedIndex,
                        //   ref rightMostChangedIndex,
                        //   boundsSyncObj);

                        RulesTools.ApplyRule(prevStep, nextStep, index, rule);
                    });

                leftMostChangedIndex = nextStepOffset;
                rightMostChangedIndex = nextStepOffset + nextStepLength - 1;

                observer.OnNext(nextStep);

                prevStep = nextStep;
            }
        }

        //private static void RunCellStep(
        //    ref BoolArrayCellSpace prevStep,
        //    ref BoolArrayCellSpace nextStep,
        //    int index,
        //    bool[]  rule,
        //    ref int? leftMostChangedIndex,
        //    ref int? rightMostChangedIndex,
        //    object boundsSyncObj)
        //{
        //    var trueOrChanged = RulesTools.ApplyRule(prevStep, nextStep, index, rule);

        //    if (trueOrChanged)
        //    {
        //        lock (boundsSyncObj)
        //        {
        //            if (!leftMostChangedIndex.HasValue || index < leftMostChangedIndex.Value)
        //                leftMostChangedIndex = index;

        //            if (!rightMostChangedIndex.HasValue || index > rightMostChangedIndex.Value)
        //                rightMostChangedIndex = index;
        //        }
        //    }
        //}
    }
}
