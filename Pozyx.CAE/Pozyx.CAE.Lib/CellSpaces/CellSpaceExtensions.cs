using System;
using System.IO;

namespace Pozyx.CAE.Lib.CellSpaces
{
    public static class CellSpaceExtensions
    {
        public static string ToCellString(this ICellSpace cellSpace, long? spaceStart = null)
        {
            if (!spaceStart.HasValue)
                spaceStart = cellSpace.Offset;

            if (spaceStart > cellSpace.Offset)
                throw new ArgumentException("Space start cannot be higher than offset.", "spaceStart");

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
    }
}
