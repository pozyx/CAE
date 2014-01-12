using System;
using System.Collections;
using System.IO;

namespace Pozyx.CAE.Lib
{
    public struct BoolArrayCellSpace : ICellSpace
    {        
        private bool[] _bools;
        
        public int Offset { get; private set; }

        public int Length
        {
            get { return _bools.Length; }
        }        

        public void Initialize(BitArray bitArray, int offset)
        {
            _bools = new bool[bitArray.Length];
            ((ICollection) bitArray).CopyTo(_bools, 0);            
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

        public string ToCellString(long? spaceStart = null)
        {
            if (!spaceStart.HasValue)
                spaceStart = Offset;

            if (spaceStart > Offset)
                throw new ArgumentException("Space start cannot be higher than offset.", "spaceStart");

            using (var sw = new StringWriter())
            {
                for (var i = spaceStart.Value; i < Offset; i++)
                    sw.Write(' ');

                for (var i = 0; i < _bools.Length; i++)
                {
                    //if ((i != 0) && (i % (sizeof(int) * 8) == 0))
                    //    sw.Write(" ");

                    sw.Write(_bools[i] ? '█' : ' ');
                }

                return sw.ToString();
            }
        }        
    }
}
