using System;
using System.Collections.Generic;
using System.Linq;

namespace Pozyx.CAE.Lib.CellSpaces
{
    public class CellSpaceEqualityComparer : IEqualityComparer<ICellSpace>
    {
        public bool Equals(ICellSpace x, ICellSpace y)
        {
            if (x == null) throw new ArgumentNullException("x");
            if (y == null) throw new ArgumentNullException("y");

            return
                x.Length == y.Length &&
                x.Offset == y.Offset &&
                Enumerable.Range(0, x.Length - 1)
                    .All(i => x.Get(x.Offset + i) == y.Get(y.Offset + i));
        }

        public int GetHashCode(ICellSpace cs)
        {
            return cs.Length ^ cs.Offset;
        }
    }
}
