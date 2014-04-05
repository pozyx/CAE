using System;
using System.Runtime.InteropServices;

namespace Pozyx.CAE.Lib.Windows
{
    // TODO: replace with real functionality

    public class Test
    {
        /// <summary>
        /// Function defined in HelloWorldLib.dll to square an array using C++ AMP
        /// </summary>
        [DllImport("Pozyx.CAE.Lib.AMP.dll", CallingConvention = CallingConvention.StdCall)]
        extern unsafe static void square_array(float* array, int length);

        public static unsafe void RunTest()
        {
            // Allocate an array
            float[] arr = new[] { 1.0f, 2.0f, 3.0f, 4.0f };

            // Square the array elements using C++ AMP
            fixed (float* arrPt = &arr[0])
            {
                square_array(arrPt, arr.Length);
            }

            // Enumerate the results
            foreach (var x in arr)
            {
                Console.WriteLine(x);
            }
        }
    }
}
