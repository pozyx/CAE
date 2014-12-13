using System;
using System.Collections;

namespace Pozyx.CAE.Lib.CellSpaces
{
    // cell space which is backed by array of which length is multiple of tile size 
    //   (requirement for tiled GPU execution)
    public abstract class PaddedArrayCellSpace<T> : ICellSpace where T : struct
    {
        // set to multiple of tile size
        private const int PadSize = 1024;

        public int Offset { get; private set; }

        public int Length { get; private set; }

        // for optimized algorithm - direct access is faster
        public T[] Cells { get; private set; }

        public virtual void Initialize(BitArray bitArray, int offset)
        {
            Length = bitArray.Length;
            Cells = new T[GetPaddedLength(bitArray.Length)];
            Offset = offset;
        }

        public void Initialize(int length, int offset)
        {
            Length = length;
            Cells = new T[GetPaddedLength(length)];
            Offset = offset;
        }

        private int GetPaddedLength(int length)
        {
            return length % PadSize == 0 ? 
                length :
                length + PadSize - (length % PadSize);
        }

        public abstract bool Get(int index);

        public abstract void Set(int index, bool value);
    }
}
