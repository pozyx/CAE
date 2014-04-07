namespace Pozyx.CAE.Lib.Runners
{
    public class OptimizedSingleThreadOneCoreCpuRunner : OptimizedSingleThreadCpuRunner
    {
        protected override void InitThread()
        {
            ThreadingTools.SetThreadProcessorAffinity(1);
        }
    }
}
