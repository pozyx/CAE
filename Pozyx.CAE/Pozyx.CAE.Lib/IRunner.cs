﻿using System;
using System.Reactive.Subjects;
using System.Threading;

namespace Pozyx.CAE.Lib
{
    public interface IRunner<out TCellSpace> where TCellSpace : ICellSpace, new()
    {
        IConnectableObservable<TCellSpace> Create(int ruleNumber, CancellationToken ct, Action threadInit = null);
    }
}
