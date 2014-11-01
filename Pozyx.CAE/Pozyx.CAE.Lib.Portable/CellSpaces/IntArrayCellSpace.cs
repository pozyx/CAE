using System;
using System.Collections;

namespace Pozyx.CAE.Lib.CellSpaces
{
    // uses int rather than bool because of accelerator restrictions
    // direct access to array is faster than through Get, Set
    public class IntArrayCellSpace : ArrayCellSpace<int>
    {
        public override void Initialize(BitArray bitArray, int offset)
        {
            base.Initialize(bitArray, offset);

            for (var i = 0; i < bitArray.Length; i++)
                Cells[i] = Convert.ToInt32(bitArray[i]);
        }

        public override bool Get(int index)
        {
            index -= Offset;

            return
                index >= 0 &&
                index < Cells.Length &&
                Convert.ToBoolean(Cells[index]);
        }

        public override void Set(int index, bool value)
        {
            index -= Offset;

            if (index < 0 || index >= Cells.Length)
                throw new Exception("Invalid index to write");

            Cells[index] = Convert.ToInt32(value);
        }
    }
}
