using System.Collections;

namespace Pozyx.CAE.Lib.CellSpaces
{
    public abstract class ArrayCellSpace<T> : ICellSpace where T : struct 
    {
        public int Offset { get; private set; }

        public int Length
        {
            get { return Cells.Length; }
        }

        // for optimized algorithm
        public T[] Cells { get; private set; }

        public virtual void Initialize(BitArray bitArray, int offset)
        {
            Offset = offset;
            Cells = new T[bitArray.Length];
        }

        public void Initialize(int length, int offset)
        {
            Cells = new T[length];
            Offset = offset;
        }

        public abstract bool Get(int index);

        public abstract void Set(int index, bool value);
    }
}
