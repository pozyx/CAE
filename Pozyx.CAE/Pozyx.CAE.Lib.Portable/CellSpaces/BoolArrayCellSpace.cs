using System;
using System.Collections;

namespace Pozyx.CAE.Lib.CellSpaces
{
    // faster than BitArrayCellSpace
    // direct access to array is faster than through Get, Set
    public class BoolArrayCellSpace : ArrayCellSpace<bool>
    {
        public override void Initialize(BitArray bitArray, int offset)
        {
           base.Initialize(bitArray, offset);
 
            ((ICollection)bitArray).CopyTo(Cells, 0);
        }

        public override bool Get(int index)
        {
            index -= Offset;

            return
                index >= 0 &&
                index < Cells.Length &&
                Cells[index];
        }

        public override void Set(int index, bool value)
        {
            index -= Offset;

            if (index < 0 || index >= Cells.Length)
                throw new Exception("Invalid index to write");

            Cells[index] = value;
        }
    }
}
