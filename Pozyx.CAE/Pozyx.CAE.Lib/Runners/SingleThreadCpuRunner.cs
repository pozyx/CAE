using Pozyx.CAE.Lib.CellSpaces;
using System;
using System.Collections;
using System.Reactive.Disposables;
using System.Reactive.Linq;
using System.Reactive.Subjects;
using System.Threading;
using System.Threading.Tasks;

namespace Pozyx.CAE.Lib.Runners
{
    public class SingleThreadCpuRunner<TCellSpace> : IRunner<TCellSpace> where TCellSpace : ICellSpace, new()
    {
        public IConnectableObservable<TCellSpace> Create(int ruleNumber, CancellationToken ct, Action threadInit = null)
        {
            var rule = RulesTools.GetBitArrayForRule(ruleNumber);

            return Observable.Create<TCellSpace>(observer =>
            {
                Task.Run(() =>
                {
                    if (threadInit != null)
                        threadInit();

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

        private static void Run(IObserver<TCellSpace> observer, BitArray rule, CancellationToken ct)
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
                nextStep.Initialize(new BitArray(nextStepLength), nextStepOffset);

                leftMostChangedIndex = null;
                rightMostChangedIndex = null;

                for (var index = nextStepOffset; index < nextStepOffset + nextStepLength; index++)
                {
                    var trueOrChanged = RulesTools.ApplyRule(prevStep, nextStep, index, rule);

                    if (trueOrChanged)
                    {
                        if (!leftMostChangedIndex.HasValue)
                            leftMostChangedIndex = index;

                        rightMostChangedIndex = index;
                    }
                }

                observer.OnNext(nextStep);

                prevStep = nextStep;
            }
        }
    }
}
