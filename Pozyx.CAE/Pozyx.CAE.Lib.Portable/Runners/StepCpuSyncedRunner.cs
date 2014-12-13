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
    public abstract class StepCpuSyncedRunner<TCellSpace> : IRunner<TCellSpace>
        where TCellSpace : ICellSpace, new()
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

        private void Run(IObserver<TCellSpace> observer, bool[] rule, CancellationToken ct)
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

                RunStep(prevStep, nextStep, rule);

                observer.OnNext(nextStep);

                CellSpaceTools.GetChangeBounds(prevStep, nextStep, out leftMostChangedIndex, out rightMostChangedIndex);

                prevStep = nextStep;
            }
        }

        protected abstract void RunStep(TCellSpace inputCellSpace, TCellSpace outputCellSpace, bool[] rule);
    }
}
