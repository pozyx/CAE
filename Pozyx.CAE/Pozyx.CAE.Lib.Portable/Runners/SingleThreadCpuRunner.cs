﻿using System;
using System.Collections;
using System.Reactive.Disposables;
using System.Reactive.Linq;
using System.Reactive.Subjects;
using System.Threading;
using System.Threading.Tasks;
using Pozyx.CAE.Lib.CellSpaces;

namespace Pozyx.CAE.Lib.Runners
{
    public class SingleThreadCpuRunner<TCellSpace> : IRunner<TCellSpace> where TCellSpace : ICellSpace, new()
    {
        protected virtual void InitThread()
        {
        }

        public IConnectableObservable<TCellSpace> Create(int ruleNumber, CancellationToken ct)
        {
            var rule = RuleTools.GetBoolArrayForRule(ruleNumber);

            return Observable.Create<TCellSpace>(observer =>
            {
                Task.Run(() =>
                {
                    InitThread();
                    Run(observer, rule, ct);
                },
                ct)
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

        private static void Run(IObserver<TCellSpace> observer, bool[] rule, CancellationToken ct)
        {
            var prevStep = new TCellSpace();
            prevStep.Initialize(new BitArray(1, true), 0);
            observer.OnNext(prevStep);

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

                var nextStep = new TCellSpace();
                nextStep.Initialize(nextStepLength, nextStepOffset);

                for (var index = nextStepOffset; index < nextStepOffset + nextStepLength; index++)
                    RuleTools.ApplyRule(prevStep, nextStep, index, rule);
               
                observer.OnNext(nextStep);

                CellSpaceTools.GetChangeBounds(prevStep, nextStep, out leftMostChangedIndex, out rightMostChangedIndex);

                prevStep = nextStep;
            }
        }

        public void Dispose()
        {
        }
    }
}
