using Pozyx.CAE.Lib.CellSpaces;

namespace Pozyx.CAE.Lib.Runners
{
    public class SingleThreadOneCoreCpuRunner<TCellSpace> : SingleThreadCpuRunner<TCellSpace> where TCellSpace : ICellSpace, new()
    {
        protected override void InitThread()
        {
            ThreadingTools.SetThreadProcessorAffinity(1);
        }
    }
}
