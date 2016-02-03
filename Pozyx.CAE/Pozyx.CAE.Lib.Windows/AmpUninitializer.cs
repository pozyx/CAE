using System.Runtime.InteropServices;

namespace Pozyx.CAE.Lib
{
    static class AmpUninitializer
    {
        [DllImport("Pozyx.CAE.Lib.AMP.dll", CallingConvention = CallingConvention.StdCall)]
        extern public static void UninitializeAmp();
    }
}
