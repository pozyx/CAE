using System;
using System.Collections;

namespace Pozyx.CAE.Lib.CellSpaces
{
    // uses int rather than bool because of accelerator restrictions
    // direct access to array is faster than through Get, Set
    // backed by array of which length is multiple of tile size 
    //   (requirement for tiled GPU execution)
    public class PaddedIntArrayCellSpace : PaddedArrayCellSpace<int>
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
                index < Length &&
                Convert.ToBoolean(Cells[index]);
        }

        public override void Set(int index, bool value)
        {
            index -= Offset;

            if (index < 0 || index >= Length)
                throw new Exception("Invalid index to write");

            Cells[index] = Convert.ToInt32(value);
        }
    }
}
