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

            var lowerBound = Math.Min(x.Offset, y.Offset);
            var length = Math.Max(x.Offset + x.Length, y.Offset + y.Length) - lowerBound;

            return Enumerable.Range(lowerBound, length - 1)
                .All(i => x.Get(i) == y.Get(i));
        }

        public int GetHashCode(ICellSpace cs)
        {
            if (cs == null) throw new ArgumentNullException("cs");

            return Enumerable.Range(0, 31)
                .Sum(i => cs.Get(i) ? Math.Pow(2, i) : 0)
                .GetHashCode();
        }
    }
}
