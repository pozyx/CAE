using System.Reactive.Subjects;
using System.Threading;
using Pozyx.CAE.Lib.CellSpaces;

namespace Pozyx.CAE.Lib.Runners
{
    public interface IRunner<out TCellSpace> where TCellSpace : ICellSpace, new()
    {
        IConnectableObservable<TCellSpace> Create(int ruleNumber, CancellationToken ct);
    }
}
