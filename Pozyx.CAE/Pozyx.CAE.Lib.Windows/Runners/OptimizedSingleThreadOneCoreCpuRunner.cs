namespace Pozyx.CAE.Lib.Runners
{
    // not useful, CPU pinning does not have any performance impact
    public class OptimizedSingleThreadOneCoreCpuRunner : OptimizedSingleThreadCpuRunner
    {
        protected override void InitThread()
        {
            ThreadingTools.SetThreadProcessorAffinity(1);
        }
    }
}
