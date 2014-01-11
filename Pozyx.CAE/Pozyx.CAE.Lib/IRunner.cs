using System.Reactive.Subjects;
using System.Threading;

namespace Pozyx.CAE.Lib
{
    public interface IRunner
    {
        IConnectableObservable<PositionedBitArray> Create(int ruleNumber, CancellationToken ct);
    }
}
