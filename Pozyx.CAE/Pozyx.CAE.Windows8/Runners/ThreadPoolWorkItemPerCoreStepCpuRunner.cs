using Pozyx.CAE.Lib.Runners;
using System;
using System.Collections.Generic;

namespace Pozyx.CAE.Windows8.Runners
{
    public class ThreadPoolWorkItemPerCoreStepCpuRunner : WorkItemPerCoreStepCpuRunner
    {
        protected override void StartWorkItemsAndWait(IList<Action> actions)
        {
            ThreadingTools.StartThreadPoolWorkItemsAndWait(actions);
        }
    }
}
