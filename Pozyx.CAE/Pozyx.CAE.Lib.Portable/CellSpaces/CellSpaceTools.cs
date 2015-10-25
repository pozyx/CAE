using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;

namespace Pozyx.CAE.Lib.CellSpaces
{
    public static class CellSpaceTools
    {
        public static string ToCellString(this ICellSpace cellSpace, long? spaceStart = null)
        {
            if (!spaceStart.HasValue)
                spaceStart = cellSpace.Offset;

            if (spaceStart > cellSpace.Offset)
                throw new ArgumentException("Space start cannot be higher than offset.", nameof(spaceStart));

            using (var sw = new StringWriter())
            {
                for (var i = spaceStart.Value; i < cellSpace.Offset; i++)
                    sw.Write(' ');

                for (var i = 0; i < cellSpace.Length; i++)
                {
                    //if ((i != 0) && (i % (sizeof(int) * 8) == 0))
                    //    sw.Write(" ");

                    sw.Write(cellSpace.Get(cellSpace.Offset + i) ? '█' : ' ');
                }

                return sw.ToString();
            }
        }

        internal static void GetChangeBounds(
            ICellSpace prevStep,
            ICellSpace nextStep,
            out int? leftMostChangedIndex,
            out int? rightMostChangedIndex)
        {
            leftMostChangedIndex = null;

            for (var i = nextStep.Offset; i < nextStep.Offset + nextStep.Length; i++)
            {
                if (prevStep.Get(i) || nextStep.Get(i))
                {
                    leftMostChangedIndex = i;
                    break;
                }
            }

            rightMostChangedIndex = null;

            for (var i = nextStep.Offset + nextStep.Length - 1; i >= nextStep.Offset; i--)
            {
                if (prevStep.Get(i) || nextStep.Get(i))
                {
                    rightMostChangedIndex = i;
                    break;
                }
            }
        }

        public class CellSpaceEqualityComparer : IEqualityComparer<ICellSpace>
        {
            public bool Equals(ICellSpace x, ICellSpace y)
            {
                var lowerBound = Math.Min(x.Offset, y.Offset);
                var length = Math.Max(x.Offset + x.Length, y.Offset + y.Length) - lowerBound;

                return Enumerable.Range(lowerBound, length - 1)
                    .All(i => x.Get(i) == y.Get(i));
            }

            public int GetHashCode(ICellSpace cs)
            {
                return Enumerable.Range(0, 31)
                    .Sum(i => cs.Get(i) ? Math.Pow(2, i) : 0)
                    .GetHashCode();
            }
        }
    }
}
