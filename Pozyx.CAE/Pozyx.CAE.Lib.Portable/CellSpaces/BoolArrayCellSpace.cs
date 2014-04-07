using System;
using System.Collections;

namespace Pozyx.CAE.Lib.CellSpaces
{
    public class BoolArrayCellSpace : ICellSpace
    {
        private bool[] _bools;

        public int Offset { get; private set; }

        public int Length
        {
            get { return _bools.Length; }
        }

        // for optimized algorithm
        public bool[] Bools
        {
            get { return _bools; }
        }

        public void Initialize(BitArray bitArray, int offset)
        {
            _bools = new bool[bitArray.Length];
            ((ICollection)bitArray).CopyTo(_bools, 0);
            Offset = offset;
        }

        public void Initialize(int length, int offset)
        {
            _bools = new bool[length];
            Offset = offset;
        }

        public bool Get(int index)
        {
            index -= Offset;

            return
                index >= 0 &&
                index < _bools.Length &&
                _bools[index];
        }

        public void Set(int index, bool value)
        {
            index -= Offset;

            if (index < 0 || index >= _bools.Length)
                throw new Exception("Invalid index to write");

            _bools[index] = value;
        }
    }
}
