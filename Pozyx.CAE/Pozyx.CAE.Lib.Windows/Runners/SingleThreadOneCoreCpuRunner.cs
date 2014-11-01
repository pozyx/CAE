using Pozyx.CAE.Lib.CellSpaces;

namespace Pozyx.CAE.Lib.Runners
{
    // not useful, CPU pinning does not have any performance impact
    public class SingleThreadOneCoreCpuRunner<TCellSpace> : SingleThreadCpuRunner<TCellSpace> where TCellSpace : ICellSpace, new()
    {
        protected override void InitThread()
        {
            ThreadingTools.SetThreadProcessorAffinity(1);
        }
    }
}
