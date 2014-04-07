using System;
using System.Threading;

namespace Pozyx.CAE.Test
{
    // http://wintellect.com/Resource-CLR-Via-CSharp-Fourth-Edition
    public static class GCNotification
    {
        private static Action<Int32> _sGCDone; // The event’s field

        public static event Action<Int32> GCDone
        {
            add
            {
                // If there were no registered delegates 
                // before, start reporting notifications now                   
                if (_sGCDone == null) { new GenObject(0); new GenObject(1); new GenObject(2); }
                _sGCDone += value;
            }
            remove { _sGCDone -= value; }
        }

        private sealed class GenObject
        {
            private readonly Int32 _mGeneration;
            public GenObject(Int32 generation) { _mGeneration = generation; }
            ~GenObject()
            { // This is the Finalize method
                // If this object is in the generation we want (or higher), 
                // notify the delegates that a GC just completed
                if (GC.GetGeneration(this) >= _mGeneration)
                {
                    var temp = Volatile.Read(ref _sGCDone);
                    if (temp != null) temp(_mGeneration);
                }

                // Keep reporting notifications if there is at least one delegate
                // registered, the AppDomain isn't unloading, and the process 
                // isn’t shutting down
                if ((_sGCDone != null) &&
                   !AppDomain.CurrentDomain.IsFinalizingForUnload() &&
                   !Environment.HasShutdownStarted)
                {
                    // For Gen 0, create a new object; for Gen 2, resurrect the object 
                    // & let the GC call Finalize again the next time Gen 2 is GC'd
                    if (_mGeneration == 0) new GenObject(0);
                    else GC.ReRegisterForFinalize(this);
                }
                else { /* Let the objects go away */ }
            }
        }
    }
}
