using Pozyx.CAE.Lib.CellSpaces;
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
    public class FakeRunner<TCellSpace> : IRunner<TCellSpace> where TCellSpace : ICellSpace, new()
    {
        public IConnectableObservable<TCellSpace> Create(int ruleNumber, CancellationToken ct)
        {
            return Observable.Create<TCellSpace>(observer =>
            {                                
                Task.Run(() => Run(observer, ct), ct)                
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

        private static void Run(IObserver<TCellSpace> observer, CancellationToken ct)
        {
            var rnd = new Random();

            while (true)
            {
                ct.ThrowIfCancellationRequested();

                var nextCellSpace = new TCellSpace();
                nextCellSpace.Initialize(
                    new BitArray(
                        Enumerable.Range(0, rnd.Next(10, 100000))
                            .Select(i => rnd.NextDouble() > 0.5)
                            .ToArray()),
                    rnd.Next(-100, 100));

                observer.OnNext(nextCellSpace);
            }
        }
    }
}
