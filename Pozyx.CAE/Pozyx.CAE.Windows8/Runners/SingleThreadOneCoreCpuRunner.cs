using Pozyx.CAE.Lib.CellSpaces;
using Pozyx.CAE.Lib.Runners;

namespace Pozyx.CAE.Windows8.Runners
{
    public class SingleThreadOneCoreCpuRunner<TCellSpace> : SingleThreadCpuRunner<TCellSpace> where TCellSpace : ICellSpace, new()
    {
        protected override void InitThread()
        {
            ThreadingTools.SetThreadProcessorAffinity(1);
        }
    }
}
